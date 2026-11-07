//! PER-MODALITY FAN-OUT (2026-11-03) — the rule of the call graph, for every modality.
//!
//! Code already has the shape every modality needs: a modality graph FILE
//! (`graphs/<modality>_<id>.json`: `nodes`, `edges`, `analysis`) with ENTRY POINTS
//! (`analysis.entry_points`, `call_sequence`) and a directed FLOW edge type (`Calls`).
//! Reading code means: start at the entries, follow the flow edges in order, and treat
//! containment as scaffolding. This module generalises that rule:
//!
//!   * every edge type of every modality is classified into an [`EdgeClass`]
//!     (Structure / Order / Flow / Spatial / Similarity / CrossModal / Provenance);
//!   * each graph gets ENTRIES — explicit when the pipeline recorded them (code), otherwise
//!     the roots (nodes with no inbound structural edge, falling back to no inbound flow edge);
//!   * the digest walks FLOW/ORDER edges breadth-first from the entries (call-sequence order),
//!     then lists the structure, spatial, similarity and cross-modal links, whole lines only.
//!
//! `span_out` packs the digest lines (most important first) until the token budget is full, so
//! a step about code sees `entries → call chain → structure`, a step about a CAD part sees the
//! parametric driver chain, a kinematic chain is read base → end effector, and so on.

use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeClass {
    /// Containment / composition scaffolding (Contains, PartOf, ChainContainsLink, HasMaterial …).
    Structure,
    /// Sequence in time/order (Follows, Precedes, TransitionsTo, FollowsStep …).
    Order,
    /// Directed dependency / data / control / causal flow (Calls, ParametricDrivesFeature, Controls …).
    Flow,
    /// Geometry / proximity (Above, Adjacent, Overlaps, LocatedIn …).
    Spatial,
    /// Likeness / correlation (SimilarTo, HomologousTo, CorrelatesWith …).
    Similarity,
    /// Link to ANOTHER modality (DescribedBy, ImplementedInCode, ParametricGeneratesMesh …).
    CrossModal,
    /// Lineage bookkeeping (DerivedFrom*, VersionOf, ForkedFrom).
    Provenance,
    Other,
}

impl EdgeClass {
    /// How strongly this class carries context during best-first span-out
    /// (multiplied with the walking score; edges carry no confidence term — present or absent).
    pub fn weight(self) -> f32 {
        match self {
            EdgeClass::Order => 0.95,
            EdgeClass::Structure => 0.9,
            EdgeClass::Flow => 0.88,
            EdgeClass::CrossModal => 0.85,
            EdgeClass::Spatial => 0.75,
            EdgeClass::Similarity => 0.7,
            EdgeClass::Other => 0.6,
            EdgeClass::Provenance => 0.35,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            EdgeClass::Structure => "structure",
            EdgeClass::Order => "order",
            EdgeClass::Flow => "flow",
            EdgeClass::Spatial => "spatial",
            EdgeClass::Similarity => "similarity",
            EdgeClass::CrossModal => "cross-modal",
            EdgeClass::Provenance => "provenance",
            EdgeClass::Other => "other",
        }
    }
}

/// Classify any modality's edge-type name (container relation or graph-file edge).
/// Explicit sets first (the survey of all 27 modality edge enums, 2026-11-03), then
/// family patterns, so a new edge type that follows the naming convention still lands
/// in the right class instead of a flat default.
pub fn classify(rel: &str) -> EdgeClass {
    use EdgeClass::*;
    let r = rel;
    // plain container-level lineage ("this was derived from that") carries real content: Flow
    if r == "DerivedFrom" || r == "Renders" {
        return if r == "Renders" { CrossModal } else { Flow };
    }
    // --- Provenance (lineage bookkeeping; deliberately weak) ---
    if r.starts_with("DerivedFrom") && !matches!(r, "DerivedFromRawWaveform" | "DerivedFromPulse" | "DerivedFromTerrain" | "DerivedFromGeometry") {
        return Provenance;
    }
    if matches!(r, "VersionOf" | "ForkedFrom" | "Unknown") {
        return Provenance;
    }
    // --- Cross-modal (links that leave the modality graph) ---
    const CROSS_EXACT: &[&str] = &[
        "DescribedBy", "DescribedByText", "Describes", "ImplementedBy", "ImplementedInCode", "RepresentedBy", "Represents",
        "VisualizedAsImage", "ParametricGeneratesMesh", "IllustratedBy", "Illustrates", "DiagramOf", "ScreenshotOf",
        "StructureOf", "AnalyzedIn", "ProvedUsing", "TranscribedAs", "TranscribedTo", "TranscribedFrom", "SynchronizedWith",
        "PlottedOnGeoMap", "GeoReferenced", "GeoreferencedAt", "KinematicChainOf", "ControlledBy", "EncodesProtein",
        "DescribesCode", "DescribesImage", "DescribesAudio", "DescribesVideo", "RecordedDuring", "ExtendedFromEEG",
        "ExtendedFromEEGGraph", "TrajectoryIn3DScene", "ContributesTo3DScene", "ContributesTo3DGeometry", "DepthFused",
        "FusedFromRadar", "FusedFromSonar", "ThermalSurface", "ThermalSurface3D", "HapticContact", "Custom",
    ];
    if CROSS_EXACT.contains(&r) || r.starts_with("LinkedTo") || r.starts_with("LinksTo") || r.starts_with("FusedWith")
        || r.starts_with("Feeds") || r.starts_with("Depicts") || r.starts_with("Commands") || r.ends_with("In3DScene")
        || r.contains("3D") && !r.starts_with("ContainedIn3D")
    {
        return CrossModal;
    }
    // --- Structure ---
    if matches!(r, "Contains" | "ContainedBy" | "PartOf" | "HasPart" | "ChildOf" | "ParentOf" | "ComposedOf" | "ContainedIn3D"
        | "ParentFeature" | "ChildFeature" | "InstanceOf" | "ChainContainsJoint" | "ChainContainsLink" | "GripContains"
        | "ContainsPOI" | "AdministrativePartOf" | "BelongsToSection" | "ConsistsOf" | "Exports" | "TypeOf" | "ParentJoint" | "ChildJoint")
        || r.starts_with("Has") || r.starts_with("ChainContains")
    {
        return Structure;
    }
    // --- Order ---
    if matches!(r, "Follows" | "Precedes" | "FollowsStep" | "FollowsIn" | "PrecedesIn" | "PrecedesInSequence" | "TransitionsTo"
        | "ContinuesFrom" | "Continues" | "PhaseFollows" | "FollowedByJoint" | "TemporalPrecedes" | "TemporalFollows"
        | "TemporallyPrecedes" | "TemporalChangeFrom" | "EvolutionOf" | "OccursDuring" | "ChangedTo" | "UpstreamOf" | "DownstreamOf")
    {
        return Order;
    }
    // --- Similarity ---
    if r.starts_with("Similar") || r.contains("Homolog") || matches!(r, "RelatesTo" | "RelatedTo" | "AlternativeTo" | "IsomerOf" | "CorrelatesWith"
        | "SameIndividualAs" | "SameEmitterAs" | "SameTrialAs" | "MatchesSignature" | "MatchesEndmember" | "Refactors" | "Tests"
        | "Generalizes" | "Specializes" | "Contradicts")
    {
        return Similarity;
    }
    // --- Spatial ---
    if matches!(r, "Above" | "Below" | "LeftOf" | "RightOf" | "AdjacentTo" | "Adjacent" | "NearTo" | "NeighborOf" | "Overlaps"
        | "OverlapsWith" | "Borders" | "Intersects" | "Within" | "Encloses" | "LocatedAt" | "LocatedIn" | "OnRoute" | "Occludes"
        | "DistanceTo" | "AdjacentFace" | "BoundsFace" | "ConnectsVertex" | "AbovePlane" | "BelowPlane" | "AdjacentCluster"
        | "SpatiallyAdjacentTo" | "AdjacentColdRegion" | "VisibleFrom" | "InSubnet" | "InVLAN" | "InSecurityZone" | "OccursIn"
        | "ContactLocatedAt" | "LocalizedAt" | "SupportedBy" | "FillsVolume" | "SurfaceGeometryOf" | "ShadowedBy" | "ObstructedBy")
    {
        return Spatial;
    }
    // --- Flow (directed dependency / data / control / causal) ---
    const FLOW_PREFIX: &[&str] = &[
        "Call", "Import", "Extend", "Implement", "DependsOn", "Depend", "Uses", "UsesVariable", "Requires", "Derives", "Defines",
        "Control", "DrivenBy", "Drives", "ParametricDrives", "FeatureCreates", "FeatureConsumes", "DrivenBySketch", "Constrain",
        "ConstrainedBy", "JointConstrains", "Route", "Forward", "Transmit", "Feedback", "Feedforward", "Transcribe", "Translate",
        "Splice", "Regulate", "Activate", "Inhibit", "Catalyz", "Phosphorylat", "Dephosphorylat", "Ubiquitinat", "Acetylat",
        "Methylat", "Cleave", "Produce", "ProductOf", "SubstrateOf", "Reacts", "Animate", "Deform", "Simulat", "MeasuredBy",
        "ActuatedBy", "Actuat", "Cause", "Enable", "Prevent", "Affect", "Perform", "Implies", "Supports", "Elaborates",
        "Summarizes", "Mates", "MatesWith", "Bonded", "BindsTo", "BoundTo", "Interacts", "Stabilizes", "OutputsTrajectory",
        "Authenticated", "ResolvedBy", "Tunneled", "Encapsulated", "Mirrored", "BlockedBy", "FirewalledBy", "LoadBalancedBy",
        "ServesOver", "ConnectedVia", "ConnectsTo", "TopologicallyConnected", "ConnectedTo", "Connects", "LinkConnects", "Links",
        "KinematicallyLinked", "TracksTarget", "DetectedAt", "DetectedIn", "Propagat", "Reflect", "Interfer", "Refract", "Scatter",
        "Beamform", "Forms", "Reconstruct", "Segmented", "Registered", "References", "Imports", "Exports", "Mentions", "Uses",
        "Binds", "Defines", "AssumesIn", "DischargesIn", "Calls", "Sync",
    ];
    if FLOW_PREFIX.iter().any(|p| r.starts_with(p)) || r.ends_with("Drives") || r.ends_with("Controls") || r.ends_with("Feeds") {
        return Flow;
    }
    heuristic_class(r)
}

/// Keyword heuristics for the long tail of domain edge names (survey of all 562 variants,
/// 2026-11-04): another modality named in the edge, spatial / order / similarity / structure
/// vocabulary, and finally the shape of a *directed domain relation* (verb / `…By|To|Of|With|From|In|At`).
fn heuristic_class(r: &str) -> EdgeClass {
    use EdgeClass::*;
    // another modality is named in the edge -> link that leaves this modality graph
    const MODALITY_TOKENS: &[&str] = &[
        "Radar", "Sonar", "Spectral", "HSI", "Thermal", "Depth", "IMU", "CAD", "BCI", "EEG", "Haptic", "Audio", "Video",
        "Image", "Chemistry", "Biology", "Protein", "Control", "Kinematic", "Geospatial", "Geo", "Weather", "Disease", "Pipeline",
        "Pulse", "Sensor", "Terrain", "Genomic", "Coding", "Codebase", "Code",
    ];
    if r.contains("Cross") || r.contains("Fused") || r.starts_with("FedBy") || r.starts_with("Anchor") || r.starts_with("Mechanism")
        || r.starts_with("Material") || r.ends_with("FromEM") || r.ends_with("ByEM") || r.ends_with("ToControl") || r.ends_with("FromBCI")
        || r.ends_with("Sonar") || r.ends_with("Radar")
        || (MODALITY_TOKENS.iter().any(|t| r.ends_with(t)) && (r.contains("From") || r.contains("To") || r.contains("With") || r.contains("In") || r.contains("Of")))
        || r.contains("126") || r.contains("Annotated") || r.contains("ContextProvides") || r.contains("AnalysisCode")
    {
        return CrossModal;
    }
    if r.contains("Correlat") || r.contains("CoExist") || r.contains("Coexpress") || r.contains("Competes") || r.contains("Colocaliz") {
        return Similarity;
    }
    if r.contains("Adjacent") || r.contains("Above") || r.contains("Below") || r.contains("Near") || r.contains("Located") || r.contains("Localized")
        || r.contains("Surface") || r.contains("Plane") || r.contains("Zone") || r.contains("Coverage") || r.contains("Range") || r.contains("Layered")
        || r.contains("Geocoded") || r.contains("Geospatial") || r.contains("Bathymetry") || r.contains("Elevation") || r.contains("Enclos") || r.contains("Gradient")
        || r.contains("Position") || r.contains("Inhabit") || r.contains("Occup")
    {
        return Spatial;
    }
    if r.contains("Phase") || r.contains("Cycle") || r.contains("Sequence") || r.contains("Keyframe") || r.contains("Trajectory") || r.contains("Temporal")
        || r.contains("Evolution") || r.contains("Timeline") || r.contains("Epoch")
    {
        return Order;
    }
    if r.contains("MadeOf") || r.contains("MemberOf") || r.contains("Contain") || r.contains("Consist") || r.contains("Composed") || r.contains("Part") || r.ends_with("Has") || r.contains("ChildOf") {
        return Structure;
    }
    // directed domain relation: verb forms and relational suffixes
    const REL_SUFFIX: &[&str] = &["By", "To", "Of", "With", "From", "In", "At", "On", "For", "Via", "Through", "Around", "Along", "Into", "Against", "As", "Under", "Over"];
    if REL_SUFFIX.iter().any(|x| r.ends_with(x)) || r.ends_with('s') || r.ends_with("ed") || r.ends_with("ing") || r.contains("_") || r.chars().all(|c| c.is_alphanumeric()) && r.len() > 3 && r.chars().next().map_or(false, |c| c.is_uppercase()) && r.chars().filter(|c| c.is_uppercase()).count() >= 2 {
        return Flow;
    }
    Other
}

/// Entry-point hints a pipeline recorded explicitly (code: `analysis.entry_points`).
fn explicit_entries(graph: &serde_json::Value) -> Vec<String> {
    graph
        .get("analysis")
        .and_then(|a| a.get("entry_points"))
        .and_then(|e| e.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

/// One rendered line of the digest, most important first.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DigestLine {
    pub text: String,
    pub class: &'static str,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct GraphDigest {
    pub modality: String,
    pub nodes: usize,
    pub edges: usize,
    pub entries: Vec<String>,
    /// Entry-rooted visit order over FLOW/ORDER edges (names), the "call sequence" of this modality.
    pub sequence: Vec<String>,
    pub class_counts: HashMap<String, usize>,
    pub lines: Vec<DigestLine>,
}

fn node_label(n: &serde_json::Value) -> String {
    let name = n.get("name").and_then(|x| x.as_str()).unwrap_or("");
    let ty = n.get("node_type").and_then(|x| x.as_str()).unwrap_or("Node");
    let id = n.get("node_id").and_then(|x| x.as_u64()).unwrap_or(0);
    if name.is_empty() {
        format!("{ty}#{id}")
    } else {
        name.to_string()
    }
}

fn props_brief(n: &serde_json::Value) -> String {
    let Some(p) = n.get("properties").and_then(|p| p.as_object()) else { return String::new() };
    let mut parts: Vec<String> = Vec::new();
    for (k, v) in p {
        let s = match v {
            serde_json::Value::String(s) if !s.is_empty() => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            _ => continue,
        };
        parts.push(format!("{k}={s}"));
    }
    parts.sort();
    parts.join(", ")
}

/// Build the modality-aware digest of a modality graph file (`{nodes, edges, analysis?}`).
/// `modality` is the file prefix (`code`, `cad`, `3d`, …) — carried for reporting; the traversal
/// rule itself is driven by edge classes, so it already covers every modality.
pub fn digest_graph(graph: &serde_json::Value, modality: &str) -> Option<GraphDigest> {
    let nodes = graph.get("nodes")?.as_array()?;
    let edges = graph.get("edges").and_then(|e| e.as_array()).cloned().unwrap_or_default();
    if nodes.is_empty() {
        return None;
    }
    let mut label: HashMap<u64, String> = HashMap::new();
    let mut by_id: HashMap<u64, &serde_json::Value> = HashMap::new();
    for n in nodes {
        let id = n.get("node_id").and_then(|x| x.as_u64()).unwrap_or(0);
        label.insert(id, node_label(n));
        by_id.insert(id, n);
    }
    struct E {
        from: u64,
        to: u64,
        ty: String,
        class: EdgeClass,
    }
    let mut es: Vec<E> = Vec::new();
    for e in &edges {
        let (Some(f), Some(t)) = (e.get("from_node").and_then(|x| x.as_u64()), e.get("to_node").and_then(|x| x.as_u64())) else { continue };
        let ty = e.get("edge_type").and_then(|x| x.as_str()).unwrap_or("Related").to_string();
        let class = classify(&ty);
        es.push(E { from: f, to: t, ty, class });
    }
    let mut class_counts: HashMap<String, usize> = HashMap::new();
    for e in &es {
        *class_counts.entry(e.class.label().to_string()).or_default() += 1;
    }

    // ENTRIES: explicit (pipeline recorded), else roots on structure edges, else roots on flow edges.
    let by_name: HashMap<String, u64> = label.iter().map(|(i, n)| (n.clone(), *i)).collect();
    let mut entry_ids: Vec<u64> = explicit_entries(graph).iter().filter_map(|n| by_name.get(n).copied()).collect();
    if entry_ids.is_empty() {
        let inbound = |classes: &[EdgeClass]| -> HashSet<u64> { es.iter().filter(|e| classes.contains(&e.class)).map(|e| e.to).collect() };
        let has_in_struct = inbound(&[EdgeClass::Structure]);
        let mut roots: Vec<u64> = by_id.keys().copied().filter(|i| !has_in_struct.contains(i)).collect();
        if roots.len() == by_id.len() || roots.is_empty() {
            let has_in_flow = inbound(&[EdgeClass::Flow, EdgeClass::Order]);
            roots = by_id.keys().copied().filter(|i| !has_in_flow.contains(i)).collect();
        }
        roots.sort();
        entry_ids = roots;
    }
    if entry_ids.is_empty() {
        entry_ids = vec![nodes[0].get("node_id").and_then(|x| x.as_u64()).unwrap_or(0)];
    }

    // SEQUENCE: BFS from the entries over FLOW/ORDER edges (then STRUCTURE for the scaffolding).
    let mut out: HashMap<u64, Vec<(u64, EdgeClass, &str)>> = HashMap::new();
    for e in &es {
        out.entry(e.from).or_default().push((e.to, e.class, e.ty.as_str()));
    }
    let mut seq_ids: Vec<u64> = Vec::new();
    let mut seen: HashSet<u64> = HashSet::new();
    let mut q: VecDeque<u64> = VecDeque::new();
    for e in &entry_ids {
        if seen.insert(*e) {
            q.push_back(*e);
        }
    }
    while let Some(n) = q.pop_front() {
        seq_ids.push(n);
        let mut next: Vec<(u64, EdgeClass)> = out.get(&n).map(|v| v.iter().map(|(t, c, _)| (*t, *c)).collect()).unwrap_or_default();
        // flow/order first, structure after, others last — a stable, class-ranked walk
        next.sort_by(|a, b| b.1.weight().partial_cmp(&a.1.weight()).unwrap_or(std::cmp::Ordering::Equal).then(a.0.cmp(&b.0)));
        for (t, c) in next {
            if matches!(c, EdgeClass::Flow | EdgeClass::Order | EdgeClass::Structure) && seen.insert(t) {
                q.push_back(t);
            }
        }
    }
    // anything unreachable from an entry goes last (still reported)
    let mut rest: Vec<u64> = by_id.keys().copied().filter(|i| !seen.contains(i)).collect();
    rest.sort();

    let name_of = |i: u64| label.get(&i).cloned().unwrap_or_else(|| format!("#{i}"));
    let entries: Vec<String> = entry_ids.iter().map(|i| name_of(*i)).collect();
    let sequence: Vec<String> = seq_ids.iter().map(|i| name_of(*i)).collect();

    let mut lines: Vec<DigestLine> = Vec::new();
    let mut push = |class: &'static str, text: String| lines.push(DigestLine { text, class });
    push("entries", format!("[{modality} graph: {} nodes, {} edges] ENTRIES: {}", nodes.len(), es.len(), entries.join(", ")));
    // FLOW/ORDER lines in sequence order (this is the "call chain")
    for id in &seq_ids {
        if let Some(v) = out.get(id) {
            for (t, c, ty) in v {
                if matches!(c, EdgeClass::Flow | EdgeClass::Order) {
                    push("flow", format!("{} -{}-> {}", name_of(*id), ty, name_of(*t)));
                }
            }
        }
    }
    // node facts in sequence order
    for id in seq_ids.iter().chain(rest.iter()) {
        if let Some(n) = by_id.get(id) {
            let ty = n.get("node_type").and_then(|x| x.as_str()).unwrap_or("Node");
            let pb = props_brief(n);
            push("node", if pb.is_empty() { format!("{ty} {}", name_of(*id)) } else { format!("{ty} {} ({pb})", name_of(*id)) });
        }
    }
    // remaining classes, strongest first
    for class in [EdgeClass::Structure, EdgeClass::CrossModal, EdgeClass::Spatial, EdgeClass::Similarity, EdgeClass::Other, EdgeClass::Provenance] {
        for e in es.iter().filter(|e| e.class == class) {
            push(class.label(), format!("{} -{}-> {}", name_of(e.from), e.ty, name_of(e.to)));
        }
    }
    Some(GraphDigest { modality: modality.to_string(), nodes: nodes.len(), edges: es.len(), entries, sequence, class_counts, lines })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classifies_edges_of_every_modality_family() {
        use EdgeClass::*;
        // code
        assert_eq!(classify("Calls"), Flow);
        assert_eq!(classify("Imports"), Flow);
        assert_eq!(classify("Contains"), Structure);
        // CAD
        assert_eq!(classify("ParametricDrivesFeature"), Flow);
        assert_eq!(classify("FeatureCreatesBody"), Flow);
        assert_eq!(classify("DrivenBySketch"), Flow);
        assert_eq!(classify("ParentFeature"), Structure);
        assert_eq!(classify("AdjacentFace"), Spatial);
        assert_eq!(classify("ParametricGeneratesMesh"), CrossModal);
        // 3D
        assert_eq!(classify("HasMaterial"), Structure);
        assert_eq!(classify("Animates"), Flow);
        assert_eq!(classify("Above"), Spatial);
        assert_eq!(classify("ImplementedInCode"), CrossModal);
        // kinematics / control / network
        assert_eq!(classify("ChainContainsLink"), Structure);
        assert_eq!(classify("FollowedByJoint"), Order);
        assert_eq!(classify("JointConstrains"), Flow);
        assert_eq!(classify("FeedbackFrom"), Flow);
        assert_eq!(classify("RoutedThrough"), Flow);
        // text / audio / video
        assert_eq!(classify("Follows"), Order);
        assert_eq!(classify("Precedes"), Order);
        assert_eq!(classify("TranscribedAs"), CrossModal);
        assert_eq!(classify("Supports"), Flow);
        // bio / chem
        assert_eq!(classify("Activates"), Flow);
        assert_eq!(classify("BondedTo"), Flow);
        assert_eq!(classify("IsomerOf"), Similarity);
        // geo / image / provenance
        assert_eq!(classify("Borders"), Spatial);
        assert_eq!(classify("Overlaps"), Spatial);
        assert_eq!(classify("DerivedFromAMT"), Provenance);
        assert_eq!(classify("SimilarTo"), Similarity);
        assert!(EdgeClass::Order.weight() > EdgeClass::Provenance.weight());
    }

    #[test]
    fn code_graph_digest_starts_at_entries_and_follows_the_call_chain() {
        let g = json!({
            "analysis": {"entry_points": ["main"]},
            "nodes": [
                {"node_id": 1, "node_type": "File", "name": "app.py", "properties": {"language": "python"}},
                {"node_id": 2, "node_type": "Function", "name": "main", "properties": {"line": 17}},
                {"node_id": 3, "node_type": "Function", "name": "parse", "properties": {}},
                {"node_id": 4, "node_type": "Function", "name": "unused", "properties": {}}
            ],
            "edges": [
                {"edge_type": "Contains", "from_node": 1, "to_node": 2, "weight": 1.0},
                {"edge_type": "Contains", "from_node": 1, "to_node": 3, "weight": 1.0},
                {"edge_type": "Contains", "from_node": 1, "to_node": 4, "weight": 1.0},
                {"edge_type": "Calls", "from_node": 2, "to_node": 3, "weight": 1.0}
            ]
        });
        let d = digest_graph(&g, "code").unwrap();
        assert_eq!(d.entries, vec!["main"]);
        // main first, then its callee; the unreachable function is still reported, last
        assert_eq!(d.sequence[0], "main");
        assert_eq!(d.sequence[1], "parse");
        assert!(d.lines[0].text.contains("ENTRIES: main"));
        assert!(d.lines.iter().any(|l| l.text == "main -Calls-> parse"));
        assert!(d.lines.iter().any(|l| l.text.starts_with("Function unused")));
    }

    #[test]
    fn graph_without_explicit_entries_uses_structural_roots() {
        // CAD-like: document contains parts; a sketch drives a feature.
        let g = json!({
            "nodes": [
                {"node_id": 1, "node_type": "CADDocument", "name": "clock.step"},
                {"node_id": 2, "node_type": "SketchNode", "name": "sketch1"},
                {"node_id": 3, "node_type": "FeatureNode", "name": "extrude1"}
            ],
            "edges": [
                {"edge_type": "Contains", "from_node": 1, "to_node": 2},
                {"edge_type": "Contains", "from_node": 1, "to_node": 3},
                {"edge_type": "ParametricDrivesFeature", "from_node": 2, "to_node": 3}
            ]
        });
        let d = digest_graph(&g, "cad").unwrap();
        assert_eq!(d.entries, vec!["clock.step"]);
        assert!(d.lines.iter().any(|l| l.class == "flow" && l.text == "sketch1 -ParametricDrivesFeature-> extrude1"));
        assert_eq!(*d.class_counts.get("flow").unwrap(), 1);
        assert_eq!(*d.class_counts.get("structure").unwrap(), 2);
    }

    #[test]
    fn empty_or_nodeless_graph_has_no_digest() {
        assert!(digest_graph(&json!({"nodes": [], "edges": []}), "x").is_none());
        assert!(digest_graph(&json!({"foo": 1}), "x").is_none());
    }

    include!("graph_fanout_edge_names.rs");

    /// Every edge-type name of every modality enum must land in a real fan-out class — "Other" is the
    /// flat-0.6 hole this module exists to close. Lists the offenders when it fails.
    #[test]
    fn every_modality_edge_name_is_classified() {
        let others: Vec<&str> = ALL_MODALITY_EDGE_NAMES.iter().copied().filter(|n| classify(n) == EdgeClass::Other).collect();
        let share = others.len() as f32 / ALL_MODALITY_EDGE_NAMES.len() as f32;
        assert!(share <= 0.02, "{:.1}% unclassified ({} of {}): {:?}", share * 100.0, others.len(), ALL_MODALITY_EDGE_NAMES.len(), others);
    }
}
