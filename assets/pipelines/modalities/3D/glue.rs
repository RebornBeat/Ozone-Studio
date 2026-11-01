//! Glue between the real readers/geometry (readers.rs, spatial.rs) and the
//! pipeline's own types (main.rs). Child module: `use super::*` reaches them.
//!
//! HONESTY RULE applied: values come from the file; unknown things are listed
//! in `processing_notes` (and `MeshInfo.metrics_absent`), never defaulted to a
//! plausible constant. UNCOMPILED pending the build gate (modality fork B).

use super::*;
use crate::readers::{self, GScene, Mesh, M4, V3};
use crate::spatial::{self, Item, Method};

const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;

fn read_file(path: &str) -> Result<Vec<u8>, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("cannot stat '{}': {}", path, e))?;
    if !meta.is_file() {
        return Err(format!("'{}' is not a regular file", path));
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err(format!("'{}' is {} bytes, over the {} byte cap", path, meta.len(), MAX_FILE_BYTES));
    }
    std::fs::read(path).map_err(|e| format!("cannot read '{}': {}", path, e))
}

fn bbox3d(mn: V3, mx: V3) -> BoundingBox3D {
    BoundingBox3D {
        min: mn,
        max: mx,
        center: [(mn[0] + mx[0]) * 0.5, (mn[1] + mx[1]) * 0.5, (mn[2] + mx[2]) * 0.5],
        size: [mx[0] - mn[0], mx[1] - mn[1], mx[2] - mn[2]],
    }
}

fn m4_to_transform(m: &M4) -> Transform4x4 {
    Transform4x4 { matrix: *m }
}

pub fn mesh_info(mesh: &Mesh, mesh_id: u64) -> MeshInfo {
    let m = readers::metrics(mesh);
    let mut absent: Vec<String> = mesh.absent.clone();
    if m.from_hint_only {
        absent.push("edge/face counts, surface area, volume and manifold status absent: vertex data was not readable (counts and bounds are the file's declared accessor values)".to_string());
    }
    if m.volume.is_none() && !m.from_hint_only {
        absent.push(if !m.closed {
            "volume absent: mesh is not closed (boundary or non-manifold edges present)".to_string()
        } else {
            "volume absent: closed but face winding is inconsistent".to_string()
        });
    }
    if m.invalid_faces > 0 {
        absent.push(format!("{} face(s) skipped: fewer than 3 vertices or an out-of-range index", m.invalid_faces));
    }
    if !mesh.has_normals {
        absent.push("no normals in the file; none were synthesized".to_string());
    }
    MeshInfo {
        mesh_id,
        name: mesh.name.clone(),
        vertex_count: m.vertex_count as u32,
        edge_count: m.edge_count as u32,
        face_count: m.face_count as u32,
        loop_count: m.loop_count as u32,
        has_uvs: mesh.has_uvs,
        uv_layer_count: if mesh.has_uvs { 1 } else { 0 },
        has_normals: mesh.has_normals,
        is_manifold: m.closed,
        has_non_manifold_edges: m.nonmanifold_edges > 0,
        polygon_type: match m.polygon {
            "triangles" => PolygonType::Triangles,
            "quads" => PolygonType::Quads,
            "ngons" => PolygonType::NGons,
            _ => PolygonType::Mixed,
        },
        bounding_box: m.aabb.map(|(a, b)| bbox3d(a, b)).unwrap_or_default(),
        surface_area: m.surface_area.unwrap_or(0.0),
        volume: m.volume,
        metrics_absent: absent,
        ..Default::default()
    }
}

fn object_from(
    id: u64,
    name: String,
    parent: Option<u64>,
    world: &M4,
    local: &M4,
    mesh_id: Option<u64>,
    world_aabb: Option<(V3, V3)>,
    object_type: ObjectType,
) -> ObjectInfo {
    let (loc, scale, euler) = readers::decompose(local);
    let mut custom = HashMap::new();
    custom.insert("local_matrix".to_string(), serde_json::json!(local));
    custom.insert("matrix_convention".to_string(), serde_json::json!("row-major, column-vector (p' = M p), translation in column 3"));
    ObjectInfo {
        object_id: id,
        name,
        object_type,
        parent_id: parent,
        data_id: mesh_id,
        // `transform` is the WORLD matrix; `location/rotation_euler/scale` decompose the LOCAL one.
        transform: m4_to_transform(world),
        location: loc,
        rotation_euler: euler,
        scale,
        dimensions: world_aabb.map(|(a, b)| [b[0] - a[0], b[1] - a[1], b[2] - a[2]]).unwrap_or([0.0; 3]),
        world_bbox: world_aabb.map(|(a, b)| bbox3d(a, b)),
        visible: true,
        custom_properties: custom,
        ..Default::default()
    }
}

fn finish(mut res: ThreeDAnalysisResult, objs: &[ObjectInfo]) -> ThreeDAnalysisResult {
    for o in objs {
        if let Some(b) = &o.world_bbox {
            res.bounding_boxes.insert(o.name.clone(), b.clone());
        }
    }
    res
}

/// AnalyzeScene: real readers for GLTF/GLB/OBJ/STL/PLY/Procedural; every other
/// source is an honest error (no placeholder analysis is returned).
pub fn analyze_scene(
    analysis_id: u64,
    data: &SceneDataSource,
    extract_materials: bool,
    extract_animation: bool,
) -> Result<ThreeDAnalysisResult, String> {
    let mut res = ThreeDAnalysisResult { analysis_id, ..Default::default() };
    match data {
        SceneDataSource::GLTF { file_path, .. } | SceneDataSource::GLB { file_path } => {
            let bytes = read_file(file_path)?;
            let dir = std::path::Path::new(file_path).parent().map(|p| p.to_path_buf());
            let sc: GScene = readers::parse_gltf(&bytes, dir.as_deref())?;
            res.source_format = if bytes.starts_with(b"glTF") { "GLB".into() } else { "GLTF".into() };
            res.source_path = Some(file_path.clone());
            res.up_axis = Axis::Y;
            res.unit_system = LengthUnit::Meters;
            res.processing_notes.push("up axis Y and unit metres are defined by the glTF 2.0 specification".to_string());
            res.processing_notes.push("scene frame range and fps are not defined by glTF scenes: left 0 (absent)".to_string());
            res.processing_notes.extend(sc.notes.iter().cloned());
            if let Some(g) = &sc.generator {
                res.processing_notes.push(format!("asset.generator: {}", g));
            }
            // meshes
            for (mi, mesh) in sc.meshes.iter().enumerate() {
                res.meshes.push(mesh_info(mesh, (mi + 1) as u64));
            }
            // objects (one per node)
            let mut objs: Vec<ObjectInfo> = Vec::new();
            // metrics once per mesh (instances share it)
            let mesh_boxes: Vec<Option<(V3, V3)>> = sc.meshes.iter().map(|m| readers::metrics(m).aabb).collect();
            for (ni, n) in sc.nodes.iter().enumerate() {
                let aabb = n.mesh.and_then(|mi| mesh_boxes[mi].map(|bb| readers::aabb_transform(&n.world, bb)));
                let ot = if n.mesh.is_some() { ObjectType::Mesh } else if n.camera { ObjectType::Camera } else { ObjectType::Empty };
                let mut o = object_from(
                    (ni + 1) as u64,
                    n.name.clone(),
                    n.parent.map(|p| (p + 1) as u64),
                    &n.world,
                    &n.local,
                    n.mesh.map(|m| (m + 1) as u64),
                    aabb,
                    ot,
                );
                o.children_ids = n.children.iter().map(|c| (*c + 1) as u64).collect();
                objs.push(o);
            }
            res.scenes.push(SceneInfo { scene_id: 1, name: sc.scene_name.clone(), ..Default::default() });
            if extract_materials {
                for (i, name) in sc.material_names.iter().enumerate() {
                    res.materials.push(MaterialInfo { material_id: (i + 1) as u64, name: name.clone(), ..Default::default() });
                }
                if !sc.material_names.is_empty() {
                    res.processing_notes.push("materials: names only; parameter values (roughness, metallic, textures) were not extracted".to_string());
                }
            }
            if extract_animation && sc.animation_count > 0 {
                res.processing_notes.push(format!("{} animation(s) declared in the file; curves not extracted by this reader", sc.animation_count));
            }
            if sc.skin_count > 0 {
                res.processing_notes.push(format!("{} skin(s) declared; armatures not extracted by this reader", sc.skin_count));
            }
            if sc.camera_count > 0 {
                res.processing_notes.push(format!("{} camera definition(s) declared; camera parameters not extracted", sc.camera_count));
            }
            res.objects = objs.clone();
            Ok(finish(res, &objs))
        }
        SceneDataSource::OBJ { file_path, .. } => {
            let text = String::from_utf8_lossy(&read_file(file_path)?).to_string();
            let meshes = readers::parse_obj(&text)?;
            res.source_format = "OBJ".into();
            res.source_path = Some(file_path.clone());
            raw_mesh_scene(&mut res, meshes, "OBJ");
            let objs = res.objects.clone();
            Ok(finish(res, &objs))
        }
        SceneDataSource::STL { file_path, unit } => {
            let mesh = readers::parse_stl(&read_file(file_path)?)?;
            res.source_format = "STL".into();
            res.source_path = Some(file_path.clone());
            res.unit_system = unit.clone();
            raw_mesh_scene(&mut res, vec![mesh], "STL");
            let objs = res.objects.clone();
            Ok(finish(res, &objs))
        }
        SceneDataSource::PLY { file_path } => {
            let mesh = readers::parse_ply(&read_file(file_path)?)?;
            res.source_format = "PLY".into();
            res.source_path = Some(file_path.clone());
            raw_mesh_scene(&mut res, vec![mesh], "PLY");
            let objs = res.objects.clone();
            Ok(finish(res, &objs))
        }
        SceneDataSource::Procedural { nodes } => {
            res.source_format = "Procedural".into();
            res.up_axis = Axis::Z;
            res.processing_notes.push("up axis not stated by procedural input: Z assumed".to_string());
            let mut objs = Vec::new();
            for (i, n) in nodes.iter().enumerate() {
                let aabb = n
                    .parameters
                    .get("bounding_box")
                    .and_then(|b| {
                        let mn = b.get("min")?.as_array()?;
                        let mx = b.get("max")?.as_array()?;
                        let g = |a: &Vec<serde_json::Value>| -> Option<V3> { Some([a.get(0)?.as_f64()?, a.get(1)?.as_f64()?, a.get(2)?.as_f64()?]) };
                        Some((g(mn)?, g(mx)?))
                    })
                    .map(|bb| readers::aabb_transform(&n.transform.matrix, bb));
                if aabb.is_none() {
                    res.processing_notes.push(format!("node '{}': no parameters.bounding_box, extent absent", n.name));
                }
                let ot = match n.node_type.to_lowercase().as_str() {
                    "mesh" => ObjectType::Mesh,
                    "light" => ObjectType::Light,
                    "camera" => ObjectType::Camera,
                    _ => ObjectType::Empty,
                };
                objs.push(object_from((i + 1) as u64, n.name.clone(), None, &n.transform.matrix, &n.transform.matrix, None, aabb, ot));
            }
            res.objects = objs.clone();
            Ok(finish(res, &objs))
        }
        SceneDataSource::URDF { .. } | SceneDataSource::MJCF { .. } | SceneDataSource::SDF { .. } | SceneDataSource::World { .. } => Err(
            "URDF/MJCF/SDF/world robot descriptions are owned by the kinematics modality (121); use its ImportURDF/ImportMJCF actions".to_string(),
        ),
        other => {
            let name = serde_json::to_value(other)
                .ok()
                .and_then(|v| v.as_object().and_then(|o| o.keys().next().cloned()))
                .unwrap_or_else(|| "unknown".to_string());
            Err(format!(
                "no reader for source '{}' in this pipeline (supported: GLTF, GLB, OBJ, STL, PLY, Procedural); nothing was fabricated",
                name
            ))
        }
    }
}

fn raw_mesh_scene(res: &mut ThreeDAnalysisResult, meshes: Vec<Mesh>, fmt: &str) {
    res.up_axis = Axis::Z;
    res.processing_notes.push(format!("{} does not define an up axis: Z assumed (CAD convention), not read from the file", fmt));
    if fmt != "STL" {
        res.processing_notes.push(format!("{} does not define a length unit: unit_system left at its default and NOT asserted", fmt));
    }
    res.processing_notes.push("scene frame range and fps are not defined by this format: left 0 (absent)".to_string());
    let mut objs = Vec::new();
    for (i, mesh) in meshes.iter().enumerate() {
        let info = mesh_info(mesh, (i + 1) as u64);
        let aabb = readers::metrics(mesh).aabb;
        objs.push(object_from((i + 1) as u64, mesh.name.clone(), None, &readers::IDENTITY, &readers::IDENTITY, Some((i + 1) as u64), aabb, ObjectType::Mesh));
        res.meshes.push(info);
    }
    res.scenes.push(SceneInfo { scene_id: 1, name: fmt.to_lowercase(), ..Default::default() });
    res.objects = objs;
}

/// AnalyzeMesh: FromFile (obj/stl/ply/glTF by extension or magic) and RawArrays.
pub fn analyze_mesh(analysis_id: u64, data: &MeshDataSource) -> Result<ThreeDAnalysisResult, String> {
    let mut res = ThreeDAnalysisResult { analysis_id, ..Default::default() };
    let meshes: Vec<Mesh> = match data {
        MeshDataSource::RawArrays { vertices, faces, normals, uvs } => {
            let mesh = Mesh {
                name: "raw_mesh".to_string(),
                positions: vertices.iter().map(|v| [v[0] as f64, v[1] as f64, v[2] as f64]).collect(),
                faces: faces.iter().map(|f| vec![f[0], f[1], f[2]]).collect(),
                has_normals: normals.is_some(),
                has_uvs: uvs.is_some(),
                uv_count: if uvs.is_some() { 1 } else { 0 },
                ..Default::default()
            };
            res.source_format = "RawArrays".into();
            vec![mesh]
        }
        MeshDataSource::FromFile { file_path, object_name } => {
            let bytes = read_file(file_path)?;
            let lower = file_path.to_lowercase();
            let mut ms: Vec<Mesh> = if lower.ends_with(".obj") {
                res.source_format = "OBJ".into();
                readers::parse_obj(&String::from_utf8_lossy(&bytes))?
            } else if lower.ends_with(".stl") {
                res.source_format = "STL".into();
                vec![readers::parse_stl(&bytes)?]
            } else if lower.ends_with(".ply") {
                res.source_format = "PLY".into();
                vec![readers::parse_ply(&bytes)?]
            } else if lower.ends_with(".gltf") || lower.ends_with(".glb") {
                res.source_format = "glTF".into();
                let dir = std::path::Path::new(file_path).parent().map(|p| p.to_path_buf());
                readers::parse_gltf(&bytes, dir.as_deref())?.meshes
            } else {
                return Err(format!("no mesh reader for '{}' (supported: .obj .stl .ply .gltf .glb)", file_path));
            };
            if let Some(name) = object_name {
                ms.retain(|m| &m.name == name);
                if ms.is_empty() {
                    return Err(format!("no mesh named '{}' in '{}'", name, file_path));
                }
            }
            res.source_path = Some(file_path.clone());
            ms
        }
        MeshDataSource::FromGraph { .. } => {
            return Err("MeshDataSource::FromGraph is not supported by AnalyzeMesh: graph meshes carry metrics already (use GetGraph)".to_string());
        }
    };
    for (i, m) in meshes.iter().enumerate() {
        res.meshes.push(mesh_info(m, (i + 1) as u64));
    }
    Ok(res)
}

// ─────────────────────────────────────────────────────────────────────────────
// GRAPH-LEVEL SPATIAL (replaces the LLM inference)
// ─────────────────────────────────────────────────────────────────────────────

/// World-space boxes of the graph's Object nodes. Nodes without a box are
/// counted in the returned skip list, never given a guessed one.
pub fn graph_items(graph: &ThreeDGraph) -> (Vec<Item>, Vec<String>) {
    let mut items = Vec::new();
    let mut skipped = Vec::new();
    for n in graph.nodes.iter().filter(|n| matches!(n.node_type, ThreeDNodeType::Object)) {
        match &n.bounding_box {
            Some(b) => items.push(Item { id: n.node_id, name: n.content.clone(), min: b.min, max: b.max }),
            None => skipped.push(format!("node {} has no world bounding box (extent unknown)", n.node_id)),
        }
    }
    (items, skipped)
}

pub fn edge_type_for(kind: &str) -> ThreeDEdgeType {
    match kind {
        "Above" => ThreeDEdgeType::Above,
        "Below" => ThreeDEdgeType::Below,
        "LeftOf" => ThreeDEdgeType::LeftOf,
        "RightOf" => ThreeDEdgeType::RightOf,
        "InFrontOf" => ThreeDEdgeType::InFrontOf,
        "Behind" => ThreeDEdgeType::Behind,
        "Intersects" => ThreeDEdgeType::Intersects,
        "Supports" => ThreeDEdgeType::Supports,
        _ => ThreeDEdgeType::NearTo,
    }
}

pub fn up_name(a: &Axis) -> &'static str {
    match a {
        Axis::X => "X",
        Axis::Y => "Y",
        Axis::Z => "Z",
    }
}

/// Geometric relations for a graph. `method`: BoundingBox | CenterOfMass; the
/// mesh-based methods (ConvexHull, MeshDistance) need vertex data the graph
/// does not carry, so they are REFUSED rather than approximated.
pub fn relations_for_graph(
    graph: &ThreeDGraph,
    up: &Axis,
    method: &SpatialMethod,
) -> Result<(Vec<(u64, u64, ThreeDEdgeType, serde_json::Value)>, Vec<String>), String> {
    let m = match method {
        SpatialMethod::BoundingBox => Method::BoundingBox,
        SpatialMethod::CenterOfMass => Method::CenterOfMass,
        SpatialMethod::ConvexHull | SpatialMethod::MeshDistance => {
            return Err("SpatialMethod ConvexHull/MeshDistance need vertex data that the graph does not store; use BoundingBox or CenterOfMass".to_string())
        }
    };
    let (items, skipped) = graph_items(graph);
    let mut notes = skipped;
    if items.len() < 2 {
        notes.push("fewer than two objects with a bounding box: no relations".to_string());
        return Ok((Vec::new(), notes));
    }
    // scale-aware tolerances
    let mut diag = 0.0f64;
    for it in &items {
        let d = ((it.max[0] - it.min[0]).powi(2) + (it.max[1] - it.min[1]).powi(2) + (it.max[2] - it.min[2]).powi(2)).sqrt();
        diag = diag.max(d);
    }
    let eps = (diag * 1e-6).max(1e-9);
    let near = (diag * 0.5).max(eps);
    let upi = spatial::up_index(up_name(up));
    let (rels, n2) = spatial::compute_relations(&items, upi, m, eps, near, 5000);
    notes.extend(n2);
    let conv = match upi {
        2 => "up=Z, right=+X, front=-Y",
        1 => "up=Y, right=+X, front=+Z",
        _ => "up=X, right=+X; front/behind and left/right not defined",
    };
    let out = rels
        .into_iter()
        .map(|r| {
            let props = serde_json::json!({
                "confidence": "derived",
                "method": format!("{:?}", method),
                "convention": conv,
                "gap": r.gap,
                "overlap_volume": r.overlap_volume,
            });
            (r.a, r.b, edge_type_for(r.kind), props)
        })
        .collect();
    Ok((out, notes))
}

/// BuildSpatialIndex: a real BVH over object boxes. Octree/KDTree are not
/// built (honest refusal); persisted next to the graph file.
pub fn build_index(graph: &ThreeDGraph, index_type: &SpatialIndexType) -> Result<(serde_json::Value, Vec<String>), String> {
    match index_type {
        SpatialIndexType::BVH => {}
        SpatialIndexType::Octree | SpatialIndexType::KDTree => {
            return Err("only the BVH index is implemented; Octree/KDTree were not built (nothing was faked)".to_string())
        }
    }
    let (items, skipped) = graph_items(graph);
    if items.is_empty() {
        return Err("no object has a bounding box: a spatial index over nothing was not built".to_string());
    }
    let bvh = spatial::build_bvh(&items, 4);
    Ok((bvh.to_json(&items, 2000), skipped))
}
