//! FILE READER — the ONE place that turns a file on disk into the input its modality pipeline takes (operator 2026-11-11: "create readers for all modals
//! with acceptable file extensions we have for each … following what we have in text … without the overengineering").
//!
//! Used by BOTH the walk's attached-file step (`graphs.rs::graph_attached_file`) and `graph_on_link.rs`, so they can no longer disagree.
//!
//! HOW EACH MODALITY TAKES A FILE (read from the pipelines, not assumed):
//!   * text 100 / code 101 / math 105 — the WHOLE file as text (`Prepared::Text`); chunking to the reading model's window happens downstream, never here.
//!   * image 102 / audio 103 / video 104 / eeg 108 / dna 107 — the pipeline opens the file BY PATH (`{"FilePath": path}`), streaming it itself.
//!     Nothing is loaded here, so there is no size ceiling and nothing to chunk in the host.
//!   * chemistry 106 — takes the molecule as text content (`MolFile` / `PDB` / `XYZ` / `SMILES` / `InChI`): read whole, tagged by extension.
//!   * 3d 109 / cad 120 / kinematics 121 / control 122 / proteomics 112 / geospatial 117 / thermal 114 / depth 115 / sound 110 — a typed data-source
//!     variant that names the file path and the format; the format comes from the extension (and, for depth, the file's own header).
//!   * every other sensor modality (imu, haptic, radar, sonar, em, hyperspectral, bci, biology, network) — its file sources carry PARAMETERS the file
//!     does not state (sample rate, band count, species, marker channel, …). A reader cannot supply them without inventing them, so the file is
//!     `Unreadable` with the exact reason; it is never given made-up parameters. A caller that knows them builds its own `Analyze` action.
//!   * uidocument 127 — a library crate with no pipeline binary: `Unreadable`.
//!
//! TWO PIPELINE INPUT/OUTPUT SHAPES exist and are kept explicit (`Shape`):
//!   Wrapped — `{"action": {"type": …}}`, analysis under the output's `result`, graph id under `result.graph_id` (image audio video chemistry dna eeg math).
//!   Bare    — the action IS the input (`{"type": …, "data": …}`), analysis under `analysis`, graph id at the top level `graph_id` (every 109–127 pipeline).
//!   Text/code are Wrapped-in-`action` too but name their fields differently (`analysis_result`, top-level `graph_id`, `link_to_existing`).
//!
//! HONESTY: unreadable = a recorded reason, never an invented analysis. Only a parameter the pipeline itself documents a default for (STL unit Meters,
//! sound channel Mono) is passed, as that default.

use serde_json::{json, Value};
use std::io::Read;

/// What a file turned into for its modality pipeline.
pub enum Prepared {
    /// The whole file as text (text / code / math). The caller builds the pipeline action with `text_analyze_input`.
    Text(String),
    /// A complete executor input (the `Analyze` action) — the pipeline reads the file itself from the path inside it.
    Action(Value),
    /// The file cannot be given to its pipeline without inventing something; the reason is recorded, no pipeline is called.
    Unreadable(String),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    /// text 100 / code 101: `{"action":{"type":"Analyze", text|code …}}` → `analysis` / top-level `graph_id`
    TextLike,
    /// math 105: `ParseExpression` → `result` / `result.graph_id`
    Math,
    /// `{"action":{…}}` in, `result` out
    Wrapped,
    /// the action is the input; `analysis` / `graph_id` at the top level
    Bare,
}

pub fn shape_of(pipeline_id: u64) -> Shape {
    match pipeline_id {
        100 | 101 => Shape::TextLike,
        105 => Shape::Math,
        102 | 103 | 104 | 106 | 107 | 108 => Shape::Wrapped,
        _ => Shape::Bare,
    }
}

fn ext_of(path: &str) -> String {
    std::path::Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}

/// The first `n` bytes of a file (a header probe — never the whole file).
fn head(path: &str, n: usize) -> Result<Vec<u8>, String> {
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; n];
    let got = f.read(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(got);
    Ok(buf)
}

// ── text-shaped modalities (the walk's existing payloads, moved here unchanged) ───────────────────────────────────────────────

/// The `Analyze` input for text (100), code (101) and math (105) from the file's text.
pub fn text_analyze_input(pipeline_id: u64, text: &str, available_modalities: &[String], path: &str) -> Value {
    if pipeline_id == 105 {
        // math has no `Analyze`: ParseExpression is its "analyze this raw content"
        return json!({"action": {"type": "ParseExpression", "expression": text, "format": "LaTeX", "extract_variables": true, "simplify": false}});
    }
    let mut action = json!({
        "type": "Analyze", "depth": "Standard", "extract_entities": true, "extract_topics": true, "extract_structure": false,
        "available_modalities": available_modalities,
    });
    action[if pipeline_id == 101 { "code" } else { "text" }] = json!(text);
    if pipeline_id == 101 {
        // the keys are required even as null (CodeModalityAction::Analyze has no serde default); the path lets code detect the language by extension
        action["language"] = Value::Null;
        action["file_path"] = json!(path);
    }
    json!({"action": action})
}

// ── CreateGraph input and graph id, per shape ─────────────────────────────────────────────────────────────────────────────────

/// The `CreateGraph` input built from the `Analyze` output. `link_to_existing` exists only on the text / code / math actions.
pub fn create_graph_input(pipeline_id: u64, analysis_output: &Value, project_id: u64) -> Value {
    match shape_of(pipeline_id) {
        Shape::Math => {
            let parse_result = analysis_output.get("result").cloned();
            // absence is 0.0, never an invented value (docs/CONTRACTS.md §5.1) — the field exists only because MathAnalysisResult declares it
            let confidence = parse_result.as_ref().and_then(|p| p.get("confidence")).and_then(|c| c.as_f64()).unwrap_or(0.0);
            json!({"action": {"type": "CreateGraph",
                "analysis": {"analysis_type": "Expression", "parse_result": parse_result, "proof_analysis": null, "confidence": confidence},
                "project_id": project_id, "link_to_existing": true}})
        }
        Shape::TextLike => json!({"action": {"type": "CreateGraph",
            "analysis_result": analysis_output.get("analysis").cloned().unwrap_or_default(),
            "project_id": project_id, "link_to_existing": true}}),
        Shape::Wrapped => json!({"action": {"type": "CreateGraph",
            "analysis": analysis_output.get("result").cloned().unwrap_or_default(), "project_id": project_id}}),
        Shape::Bare => json!({"type": "CreateGraph",
            "analysis": analysis_output.get("analysis").cloned().unwrap_or_default(), "project_id": project_id}),
    }
}

/// The persisted graph id the pipeline returned — never a minted stand-in; an absent id is a failure the caller records.
pub fn graph_id_of(pipeline_id: u64, created: &Value) -> Option<u64> {
    match shape_of(pipeline_id) {
        Shape::Math | Shape::Wrapped => created.get("result").and_then(|r| r.get("graph_id")).and_then(|g| g.as_u64()),
        Shape::TextLike | Shape::Bare => created.get("graph_id").and_then(|g| g.as_u64()),
    }
}

// ── the readers ───────────────────────────────────────────────────────────────────────────────────────────────────────────────

/// Prepare `path` for its modality pipeline. `pipeline_id` is the modality's pipeline (0 → already mapped to text by the caller).
pub async fn prepare(path: &str, pipeline_id: u64) -> Prepared {
    let ext = ext_of(path);
    match pipeline_id {
        100 | 101 | 105 => match tokio::fs::read_to_string(path).await {
            Ok(t) => Prepared::Text(t),
            // binary documents (pdf, docx, …) reach text by extension; their text needs an extractor, not a UTF-8 read
            Err(e) => Prepared::Unreadable(format!("not readable as UTF-8 text ({e}); a binary document needs its text extractor, not a plain read")),
        },
        106 => chemistry(path, &ext).await,
        _ => path_based(path, &ext, pipeline_id),
    }
}

fn wrapped(action: Value) -> Prepared {
    Prepared::Action(json!({ "action": action }))
}

fn bare(action: Value) -> Prepared {
    Prepared::Action(action)
}

async fn chemistry(path: &str, ext: &str) -> Prepared {
    let variant = match ext {
        "mol" | "sdf" => "MolFile",
        "pdb" => "PDB",
        "xyz" => "XYZ",
        "smi" | "smiles" => "SMILES",
        "inchi" => "InChI",
        _ => return Prepared::Unreadable(format!("chemistry takes MolFile / PDB / XYZ / SMILES / InChI content; there is no input for .{ext} (cif, mol2, cml, cdx, cdxml, rxn, ct, mae, maegz)")),
    };
    match tokio::fs::read_to_string(path).await {
        Ok(text) => wrapped(json!({"type": "AnalyzeMolecule", "molecule": { variant: text }, "compute_properties": true, "find_functional_groups": true, "generate_3d": false})),
        Err(e) => Prepared::Unreadable(format!("not readable as text ({e}); .{ext} is read as {variant} content")),
    }
}

fn path_based(path: &str, ext: &str, pipeline_id: u64) -> Prepared {
    match pipeline_id {
        102 => wrapped(json!({"type": "Analyze", "image_data": {"FilePath": path}})),
        103 => wrapped(json!({"type": "Analyze", "audio_data": {"FilePath": path}})),
        104 => wrapped(json!({"type": "Analyze", "video_data": {"FilePath": path}})),
        108 => wrapped(json!({"type": "Analyze", "eeg_data": {"FilePath": path}, "detect_events": true, "extract_bands": true, "detect_artifacts": true})),
        107 => match ext {
            "fasta" | "fa" | "fna" | "ffn" | "faa" | "frn" | "fastq" | "fq" | "gbk" | "gb" | "genbank" => {
                wrapped(json!({"type": "AnalyzeSequence", "sequence": {"FilePath": path}, "compute_statistics": true}))
            }
            _ => Prepared::Unreadable(format!("AnalyzeSequence reads sequences (fasta / fastq / genbank); .{ext} is an annotation, variant, alignment or track file with no sequence input")),
        },
        109 => scene_3d(path, ext),
        110 => bare(json!({"type": "Analyze", "data": {"AudioFile": {"file_path": path, "channel": "Mono"}},
            "detect_vocalizations": true, "identify_species": true, "decompose_units": true, "reconstruct_scene": true, "extract_signatures": true})),
        112 => match ext {
            "mmcif" => bare(proteomics(json!({"MMCIF": {"file_path": path, "entry_id": null}}))),
            _ => Prepared::Unreadable(format!("proteomics .{ext} needs parameters the file does not state (mass-spec format and experiment type, or map/density metadata)")),
        },
        114 => match ext {
            "rjpeg" | "fff" => bare(json!({"type": "Analyze", "data": {"FLIRRadiometric": {"file_path": path, "extract_embedded_metadata": true}},
                "detect_hotspots": true, "detect_coldspots": true, "compute_gradients": true, "classify_materials": true, "detect_anomalies": true})),
            _ => Prepared::Unreadable(format!("thermal .{ext} needs parameters the file does not state (frame rate, raw ADC geometry and radiometric coefficients); only radiometric FLIR files are self-describing")),
        },
        115 => depth(path, ext),
        117 => geospatial(path, ext),
        120 => cad(path, ext),
        121 => kinematics(path, ext),
        122 => match ext {
            "slx" | "mdl" | "simulink" => bare(json!({"type": "Analyze", "data": {"Simulink": {"file_path": path, "subsystem_path": null}},
                "analyze_stability": true, "analyze_performance": true, "analyze_robustness": true, "identify_system": true})),
            _ => Prepared::Unreadable(format!("control .{ext} needs the model name or system variable, which the file does not state; only Simulink models (.slx / .mdl) are self-describing")),
        },
        116 | 113 | 124 | 125 | 118 | 126 | 119 | 111 => Prepared::Unreadable(sensor_parameters(pipeline_id, ext)),
        123 => Prepared::Unreadable(format!("network sources are scan / capture / flow / config dumps (nmap, pcap, netflow, snmp, bgp, cloud, lldp, zeek, firewall); there is no input for a .{ext} graph file")),
        127 => Prepared::Unreadable("uidocument is a library crate with no pipeline binary; no graph can be created for it".to_string()),
        _ => Prepared::Unreadable(format!("no reader for pipeline {pipeline_id} (.{ext})")),
    }
}

fn sensor_parameters(pipeline_id: u64, ext: &str) -> String {
    let (name, needs) = match pipeline_id {
        116 => ("imu", "column map and units, or the ROS topic and version, or the sample rate"),
        113 => ("haptic", "sensor model, array geometry, sample rate"),
        124 => ("radar", "sample rate and carrier frequency, or the SAR format, polarization and orbit"),
        125 => ("sonar", "format and frequency, beam count and swath"),
        118 => ("electromagnetic", "sample rate, centre frequency and gain"),
        126 => ("hyperspectral", "band count, wavelength range and resolution"),
        119 => ("bci", "marker channel and paradigm, or electrode count and sample rate"),
        _ => ("biology", "species / organism and the data format"),
    };
    format!("{name} .{ext} needs parameters the file does not state ({needs}); a reader would have to invent them, so none is built — a caller that knows them sends its own Analyze action")
}

fn scene_3d(path: &str, ext: &str) -> Prepared {
    let src = match ext {
        "gltf" => json!({"GLTF": {"file_path": path, "load_animations": true, "load_materials": true}}),
        "glb" => json!({"GLB": {"file_path": path}}),
        "obj" => json!({"OBJ": {"file_path": path, "mtl_path": null}}),
        // an STL carries no unit; Meters is the pipeline's own documented default (`LengthUnit` #[default])
        "stl" => json!({"STL": {"file_path": path, "unit": "Meters"}}),
        "ply" => json!({"PLY": {"file_path": path}}),
        // glue.rs::analyze_scene reads exactly these five (and says so for the rest): fbx, usd*, blend, dae, world, 3ds, c4d, ma, mb have no reader yet
        _ => return Prepared::Unreadable(format!("the 3D pipeline reads glTF / GLB / OBJ / STL / PLY; .{ext} has no reader in it yet (it refuses honestly rather than fabricate)")),
    };
    bare(json!({"type": "AnalyzeScene", "data": src, "extract_hierarchy": true, "extract_physics": false, "extract_animation": true, "extract_materials": true}))
}

fn cad(path: &str, ext: &str) -> Prepared {
    let variant = match ext {
        "step" | "stp" => "STEP",
        "iges" | "igs" => "IGES",
        "brep" => "BREP",
        "occ" => "OCC",
        "f3d" => "F3D",
        "fcstd" => "FCStd",
        "sldprt" => "SLDPRT",
        "sldasm" => "SLDASM",
        "x_t" => "X_T",
        "x_b" => "X_B",
        "sat" => "SAT",
        _ => return Prepared::Unreadable(format!("the CAD pipeline has no source for .{ext}")),
    };
    bare(json!({"type": "AnalyzeFile", "data": { variant: {"file_path": path} }, "extract_features": true, "extract_assembly": true, "extract_tolerances": true}))
}

fn kinematics(path: &str, ext: &str) -> Prepared {
    let variant = match ext {
        "urdf" => "URDF",
        "mjcf" => "MJCF",
        "sdf" => "SDF",
        "bvh" => "BVH",
        "c3d" => "C3D",
        "osim" => "OSIM",
        _ => return Prepared::Unreadable(format!("kinematics .{ext} (a DH table) is a parameter list, not a chain file; the pipeline takes URDF / MJCF / SDF / BVH / C3D / OSIM")),
    };
    bare(json!({"type": "AnalyzeChain", "data": { variant: {"file_path": path} }, "compute_workspace": true, "detect_singularities": true}))
}

fn proteomics(data: Value) -> Value {
    json!({"type": "Analyze", "data": data, "extract_structure": true, "extract_interactions": true, "extract_modifications": true, "extract_expression": true, "extract_function": true})
}

fn geospatial(path: &str, ext: &str) -> Prepared {
    let data = match ext {
        "geojson" => json!({"GeoJSON": {"file_path": path}}),
        "kml" | "kmz" => json!({"KML": {"file_path": path}}),
        "gpx" => json!({"GPX": {"file_path": path}}),
        "shp" => json!({"Shapefile": {"file_path": path, "crs": null}}),
        "osm" | "pbf" => json!({"OSM": {"file_path": path, "extract_features": []}}),
        "dbf" | "shx" | "prj" => return Prepared::Unreadable(format!(".{ext} is a shapefile sidecar; its .shp is the source and carries it")),
        _ => return Prepared::Unreadable(format!("geospatial .{ext} needs parameters the file does not state (vertical datum and CRS, band interpretation, or the variable name)")),
    };
    bare(json!({"type": "Analyze", "data": data, "extract_features": true, "build_topology": true, "compute_terrain": true, "extract_routes": true}))
}

/// Depth: the LiDAR / PCD flags are properties of the file, so they are read from its own header, never guessed.
fn depth(path: &str, ext: &str) -> Prepared {
    let data = match ext {
        "las" | "laz" => {
            let h = match head(path, 256) {
                Ok(h) => h,
                Err(e) => return Prepared::Unreadable(format!("LAS header not readable: {e}")),
            };
            if h.len() < 105 || &h[0..4] != b"LASF" {
                return Prepared::Unreadable("not a LAS/LAZ file: the LASF signature is missing".to_string());
            }
            // point data record format (byte 104; bit 7 / 6 are LAZ compression markers); every format carries intensity and classification
            let fmt = h[104] & 0x3f;
            let has_rgb = matches!(fmt, 2 | 3 | 5 | 7 | 8 | 10);
            json!({"LiDARFile": {"file_path": path, "format": if ext == "laz" { "LAZ" } else { "LAS" },
                "has_intensity": true, "has_rgb": has_rgb, "has_classification": true, "coordinate_system": "SensorLocal"}})
        }
        "pcd" => {
            let h = match head(path, 4096) {
                Ok(h) => h,
                Err(e) => return Prepared::Unreadable(format!("PCD header not readable: {e}")),
            };
            let text = String::from_utf8_lossy(&h).to_string();
            let mut fields = String::new();
            let mut data_kind: Option<String> = None;
            for line in text.lines() {
                let l = line.trim();
                if let Some(rest) = l.strip_prefix("FIELDS") {
                    fields = rest.to_ascii_lowercase();
                } else if let Some(rest) = l.strip_prefix("DATA") {
                    data_kind = Some(rest.trim().to_ascii_lowercase());
                    break;
                }
            }
            let Some(kind) = data_kind else { return Prepared::Unreadable("PCD header has no DATA line".to_string()) };
            json!({"PCDFile": {"file_path": path, "binary": kind != "ascii",
                "has_normals": fields.contains("normal_x"), "has_rgb": fields.split_whitespace().any(|f| f == "rgb" || f == "rgba")}})
        }
        _ => return Prepared::Unreadable(format!("depth .{ext} needs parameters the file does not state (scan count, delimiter and columns, or camera intrinsics); only LAS / LAZ / PCD carry their own header")),
    };
    bare(json!({"type": "Analyze", "data": data, "extract_surfaces": true, "extract_objects": true, "extract_normals": true, "compute_topology": true, "compute_occlusion": false}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_cover_every_pipeline() {
        assert_eq!(shape_of(100), Shape::TextLike);
        assert_eq!(shape_of(105), Shape::Math);
        assert_eq!(shape_of(102), Shape::Wrapped);
        assert_eq!(shape_of(109), Shape::Bare);
        assert_eq!(shape_of(127), Shape::Bare);
    }

    #[test]
    fn create_graph_and_graph_id_follow_the_pipeline_shape() {
        let wrapped = create_graph_input(102, &json!({"result": {"width": 3}}), 5);
        assert_eq!(wrapped["action"]["analysis"]["width"], 3);
        assert!(wrapped["action"].get("link_to_existing").is_none());
        let bare = create_graph_input(109, &json!({"analysis": {"nodes": 1}}), 5);
        assert_eq!(bare["type"], "CreateGraph");
        assert_eq!(bare["analysis"]["nodes"], 1);
        assert_eq!(graph_id_of(102, &json!({"result": {"graph_id": 4}})), Some(4));
        assert_eq!(graph_id_of(109, &json!({"graph_id": 8})), Some(8));
        assert_eq!(graph_id_of(109, &json!({})), None);
    }

    #[test]
    fn path_readers_name_the_file_and_never_invent_parameters() {
        let Prepared::Action(a) = path_based("/x/a.png", "png", 102) else { panic!("image") };
        assert_eq!(a["action"]["image_data"]["FilePath"], "/x/a.png");
        let Prepared::Action(a) = path_based("/x/m.obj", "obj", 109) else { panic!("3d") };
        assert_eq!(a["type"], "AnalyzeScene");
        assert_eq!(a["data"]["OBJ"]["file_path"], "/x/m.obj");
        // a sensor file whose parameters are not in the file is recorded, not given made-up ones
        assert!(matches!(path_based("/x/a.iq", "iq", 118), Prepared::Unreadable(_)));
        assert!(matches!(path_based("/x/a.3ds", "3ds", 109), Prepared::Unreadable(_)));
        assert!(matches!(path_based("/x/a.html", "html", 127), Prepared::Unreadable(_)));
    }
}
