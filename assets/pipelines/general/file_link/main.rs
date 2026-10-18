//! FileLinkPipeline - Pipeline #30
//! Link local files to projects. Files are referenced, not copied.
//!
//! STORAGE: Persists file references to JSON files in the data directory
//! Called FROM WorkspaceTab UI - does NOT need its own tab

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[path = "../../shared/ozone_serve.rs"]
mod ozone_serve;

// Same real-ZSEI-over-HTTP pattern as text/code modality and
// context_aggregation — ZSEIQuery is externally-tagged, wire format
// {"VariantName": {fields...}}.
fn ozone_host() -> String {
    std::env::var("OZONE_HOST").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string())
}

async fn zsei_query(query: serde_json::Value) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/zsei/query", ozone_host()))
        .json(&serde_json::json!({"query": query, "session_token": ""}))
        .send()
        .await
        .map_err(|e| format!("zsei query request failed: {}", e))?;
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("zsei query response parse failed: {}", e))?;
    if body.get("success").and_then(|s| s.as_bool()) != Some(true) {
        return Err(body
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("zsei query failed")
            .to_string());
    }
    Ok(body.get("result").cloned().unwrap_or(serde_json::Value::Null))
}

/// Dispatch a file to its real detected-modality pipeline for analysis, via
/// the host's own generic `POST /pipeline/execute` route (the same route
/// `window.ozone`/the UI's `executePipeline()` already calls) — NOT by
/// shelling out to the sibling pipeline binary directly. file_link is its
/// own independent process (confirmed this session: pipeline crates share no
/// library with `src/pipeline/executor.rs`), so letting the host's real
/// `PipelineExecutor` do the actual subprocess spawn/path-resolution/gate
/// logic avoids duplicating all of that here — same pattern `zsei_query`
/// above already established for ZSEI calls, just a different host route.
/// Returns the real pipeline's output `data` map on success.
async fn invoke_modality_pipeline(
    pipeline_id: u64,
    action_data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let input = serde_json::json!({
        "data": action_data,
        "context": {
            "user_id": 0,
            "device_id": 0,
            "workspace_id": null,
            "project_id": null,
            "task_context_id": null,
            "metadata": {}
        }
    });
    let resp = client
        .post(format!("{}/pipeline/execute", ozone_host()))
        .json(&serde_json::json!({
            "pipeline_id": pipeline_id,
            "input": input,
            "session_token": ""
        }))
        .send()
        .await
        .map_err(|e| format!("pipeline execute request failed: {}", e))?;
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("pipeline execute response parse failed: {}", e))?;
    if body.get("success").and_then(|s| s.as_bool()) != Some(true) {
        return Err(body
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("pipeline execution failed")
            .to_string());
    }
    Ok(body.get("output").cloned().unwrap_or(serde_json::Value::Null))
}

/// Real pipeline ids for the modalities wired to a real dispatch below —
/// confirmed against `src/orchestrator/graphs.rs`'s own modality→id map and
/// `src/bootstrap.rs`'s registered pipeline entries, not guessed.
fn modality_pipeline_id(modality: &str) -> Option<u64> {
    match modality {
        "Image" => Some(102),
        "Audio" => Some(103),
        "Video" => Some(104),
        // Text/Code confirmed against `src/orchestrator/graphs.rs`'s real
        // `modality_name_to_pipeline_id` map (100/101) — previously these
        // fell into the fallback match below, which did crude inline
        // line/word counting and NEVER actually called the real pipeline
        // despite comments claiming otherwise ("In real implementation,
        // this would call the pipeline executor"). Wired for real now,
        // same dispatch path as Image/Audio/Video.
        "Text" => Some(100),
        "Code" => Some(101),
        // Group-A wiring (2026-10-17): real pipeline ids confirmed against
        // `src/orchestrator/graphs.rs`'s own modality->id map (100-126).
        // DNA/EEG/Math/Chemistry are wired below for ALL of their existing
        // `detect_modality` extensions; 3D/CAD/Kinematics get NEW
        // `detect_modality` buckets added below (previously undetectable —
        // this function could never have returned those strings); Control
        // is deliberately narrow (only ".slx", see `detect_modality`) since
        // every other `ControlDataSource` variant needs a required field
        // (e.g. `model_name`, `sample_rate_hz`) this file has no honest way
        // to supply. Biology/Proteomics/Depth/Electromagnetic are NOT
        // wired — investigated and skipped, see the long comment on
        // `analyze_action_payload` below for why each is genuinely
        // infeasible without fabricating required domain metadata.
        "DNA" => Some(107),
        "EEG" => Some(108),
        "Math" => Some(105),
        "Chemistry" => Some(106),
        "3D" => Some(109),
        "CAD" => Some(120),
        "Kinematics" => Some(121),
        "Control" => Some(122),
        _ => None,
    }
}

/// The `{"type": "Analyze", ...}` payload for each wired modality's real
/// `Analyze` action, keyed on its own real `*Data`/`*_data` field name and
/// `FilePath` variant (confirmed directly against each pipeline's own enum
/// this session — image/audio/video all already use `#[serde(tag = "type")]`
/// and a `FilePath(String)` data-source variant, so no envelope/tag fix was
/// needed here unlike the 18 modalities the contract-unification pass
/// covered).
fn analyze_action_payload(modality: &str, file_path: &str) -> Option<serde_json::Value> {
    match modality {
        "Image" => Some(serde_json::json!({
            "type": "Analyze",
            "image_data": {"FilePath": file_path}
        })),
        "Audio" => Some(serde_json::json!({
            "type": "Analyze",
            "audio_data": {"FilePath": file_path}
        })),
        "Video" => Some(serde_json::json!({
            "type": "Analyze",
            "video_data": {"FilePath": file_path}
        })),
        // Text/Code's real `Analyze` actions take inline content fields
        // (`text`/`code`), not a `FilePath` data-source variant like
        // image/audio/video — confirmed directly against
        // `assets/pipelines/modalities/{text,code}/main.rs`'s own
        // `*ModalityAction::Analyze` enum. So these two read the file's
        // real content here (text-safe: both modalities are only ever
        // detected for text-based extensions). A read failure returns
        // `None`, same honest "no payload" path as an unwired modality.
        "Text" => {
            let text = fs::read_to_string(file_path).ok()?;
            Some(serde_json::json!({
                "type": "Analyze",
                "text": text
            }))
        }
        "Code" => {
            let code = fs::read_to_string(file_path).ok()?;
            Some(serde_json::json!({
                "type": "Analyze",
                "code": code,
                "language": null,
                "file_path": file_path
            }))
        }
        // DNA: `SequenceData::FilePath(String)` (confirmed in
        // `assets/pipelines/modalities/dna/main.rs`) accepts a bare path for
        // ANY of this file's DNA extensions — the real pipeline does its own
        // format sniffing (FASTA/GenBank/FASTQ/etc.) from there, same as
        // image/audio/video's FilePath variant above.
        "DNA" => Some(serde_json::json!({
            "type": "AnalyzeSequence",
            "sequence": {"FilePath": file_path}
        })),
        // EEG: `EEGData::FilePath(String)`, same reasoning as DNA. The other
        // 3 `Analyze` fields (`depth`/`detect_events`/`extract_bands`) all
        // carry `#[serde(default)]` in the real enum, confirmed — safe to
        // omit entirely rather than guess a non-default value.
        "EEG" => Some(serde_json::json!({
            "type": "Analyze",
            "eeg_data": {"FilePath": file_path}
        })),
        // Chemistry: `MoleculeData` has NO generic FilePath/content variant
        // covering every chemistry extension — only PDB(String) and
        // MolFile(String) (both take real file CONTENT, confirmed against
        // `assets/pipelines/modalities/chemistry/main.rs`) map honestly onto
        // an extension this file already detects. xyz/cif/cml/cdx/cdxml/rxn
        // (also bucketed under "Chemistry" by `detect_modality`) have no
        // matching variant — `MoleculeData::Formula` means a textual
        // molecular formula like "C6H12O6", not raw file content, so using
        // it here would mislabel the data; `AnalyzeReaction`'s `ReactionData`
        // needs pre-parsed `Vec<MoleculeData>` reactant/product lists this
        // file cannot honestly produce from bytes alone. Returns `None` for
        // those extensions — falls through to the honest generic fallback
        // below, not an error (see the `analyze_file` dispatch-gate fix).
        "Chemistry" => {
            let ext = std::path::Path::new(file_path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let content = fs::read_to_string(file_path).ok()?;
            match ext.as_str() {
                "pdb" => Some(serde_json::json!({
                    "type": "AnalyzeMolecule",
                    "molecule": {"PDB": content}
                })),
                "mol" | "sdf" | "mol2" => Some(serde_json::json!({
                    "type": "AnalyzeMolecule",
                    "molecule": {"MolFile": content}
                })),
                _ => None,
            }
        }
        // Math: `AnalyzeProof` takes `proof: String` (real file content, not
        // a path — confirmed against `assets/pipelines/modalities/math/
        // main.rs`) plus a `format: MathFormat` hint; `verify`/
        // `check_completeness` both carry `#[serde(default)]`, omitted. Format
        // is a best-effort, non-fabricated mapping from real extension to the
        // real `MathFormat` enum's variants — `tex`/`latex`/`rmd`/`qmd` really
        // are LaTeX-flavored; `nb`/`mw` really are Mathematica notebooks;
        // `mpl`/`maple` really are Maple. `sage`/`wxm`/`wxmx`/`ipynb` have no
        // matching `MathFormat` variant (Sage/wxMaxima/Jupyter aren't LaTeX,
        // Mathematica, or Maple) — `Plain` is the honest "no dedicated parser
        // for this format" choice, not a guess at which of the 3 it resembles.
        "Math" => {
            let content = fs::read_to_string(file_path).ok()?;
            let ext = std::path::Path::new(file_path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let format = match ext.as_str() {
                "tex" | "latex" | "rmd" | "qmd" => "LaTeX",
                "nb" | "mw" => "Mathematica",
                "mpl" | "maple" => "Maple",
                _ => "Plain", // sage/wxm/wxmx/ipynb: no dedicated real variant
            };
            Some(serde_json::json!({
                "type": "AnalyzeProof",
                "proof": content,
                "format": format
            }))
        }
        // 3D: no `detect_modality` bucket existed for this at all before
        // this pass (confirmed — only Code/Text/Data/Config/Web/Document/
        // Image/Audio/Video/Math/Chemistry/DNA/EEG/Archive were detected;
        // 3D/CAD/Kinematics files were all silently falling into
        // "Unknown"). `MeshDataSource::FromFile{file_path, object_name:
        // None}` (confirmed in `assets/pipelines/modalities/3D/main.rs`) is
        // genuinely generic across every mesh sub-format, unlike
        // `SceneDataSource` which is extension-tagged per format (GLTF/OBJ/
        // FBX/...) — using the generic one avoids guessing a specific
        // sub-format tag wrong. `compute_normals`/`compute_uv`/
        // `compute_topology` have no `#[serde(default)]` (confirmed) — set
        // `true` for all 3 as a reasonable "extract everything available"
        // request parameter, not a factual claim about the file's content.
        "3D" => Some(serde_json::json!({
            "type": "AnalyzeMesh",
            "data": {"FromFile": {"file_path": file_path, "object_name": null}},
            "compute_normals": true,
            "compute_uv": true,
            "compute_topology": true
        })),
        // CAD: new `detect_modality` bucket (see below); `CADDataSource` is
        // purely extension-tagged (STEP/IGES/BREP/OCC/F3D/FCStd/SLDPRT/
        // SLDASM, each `{file_path: String}` only, confirmed in
        // `assets/pipelines/modalities/CAD/main.rs`) — extension maps 1:1 to
        // variant, no guessing. `extract_features`/`extract_assembly`/
        // `extract_tolerances` have no default — `true` for all 3, same
        // "extract everything" reasoning as 3D above.
        "CAD" => {
            let ext = std::path::Path::new(file_path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let variant = match ext.as_str() {
                "step" | "stp" => "STEP",
                "iges" | "igs" => "IGES",
                "brep" => "BREP",
                "occ" => "OCC",
                "f3d" => "F3D",
                "fcstd" => "FCStd",
                "sldprt" => "SLDPRT",
                "sldasm" => "SLDASM",
                _ => return None,
            };
            // `variant` is a runtime &str, not a literal — `json!({variant:
            // ...})` would NOT substitute its value as the key (the macro
            // needs a literal token there), so the tagged inner object is
            // built explicitly via `serde_json::Map` instead.
            let mut data_obj = serde_json::Map::new();
            data_obj.insert(
                variant.to_string(),
                serde_json::json!({"file_path": file_path}),
            );
            Some(serde_json::json!({
                "type": "AnalyzeFile",
                "data": serde_json::Value::Object(data_obj),
                "extract_features": true,
                "extract_assembly": true,
                "extract_tolerances": true
            }))
        }
        // Kinematics: new `detect_modality` bucket (see below);
        // `KinematicsDataSource`'s file-based variants (URDF/MJCF/SDF/BVH/
        // C3D/OSIM, each `{file_path: String}` only, confirmed in
        // `assets/pipelines/modalities/kinematics/main.rs`) map 1:1 to
        // extension, same reasoning as CAD. `compute_workspace`/
        // `detect_singularities` have no default — `true` for both.
        "Kinematics" => {
            let ext = std::path::Path::new(file_path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let variant = match ext.as_str() {
                "urdf" => "URDF",
                "mjcf" => "MJCF",
                "sdf" => "SDF",
                "bvh" => "BVH",
                "c3d" => "C3D",
                "osim" => "OSIM",
                _ => return None,
            };
            // Same dynamic-key fix as CAD above — `variant` is a runtime
            // &str, built explicitly via `serde_json::Map` rather than
            // inside the `json!` macro's literal-key object syntax.
            let mut data_obj = serde_json::Map::new();
            data_obj.insert(
                variant.to_string(),
                serde_json::json!({"file_path": file_path}),
            );
            Some(serde_json::json!({
                "type": "AnalyzeChain",
                "data": serde_json::Value::Object(data_obj),
                "compute_workspace": true,
                "detect_singularities": true
            }))
        }
        // Control: deliberately narrow — of `ControlDataSource`'s variants,
        // only `MatlabFile{file_path, system_variable: Option}` and
        // `Simulink{file_path, subsystem_path: Option}` have no OTHER
        // required field this file would have to fabricate (every other
        // variant needs something real-but-unknowable: `model_name`,
        // `sample_rate_hz`, 4 separate matrix files, etc. — confirmed in
        // `assets/pipelines/modalities/control/main.rs`). `.m` is already
        // claimed by "Math" in `detect_modality` (Mathematica scripts) so
        // MatlabFile is unreachable without a real conflict; only Simulink's
        // `.slx` is wired, via a new, non-conflicting `detect_modality`
        // bucket below.
        "Control" => Some(serde_json::json!({
            "type": "Analyze",
            "data": {"Simulink": {"file_path": file_path, "subsystem_path": null}},
            "analyze_stability": true,
            "analyze_performance": true,
            "analyze_robustness": true,
            "identify_system": true
        })),
        // Biology/Proteomics/Depth/Electromagnetic: investigated, NOT
        // wired — genuinely infeasible without fabricating required data:
        // - Biology: every `BiologyDataSource` variant needs a required
        //   `format`-type enum AND `organism: String` with no honest
        //   default derivable from a bare file.
        // - Proteomics: every real extension (fasta/fa, pdb, cif) is
        //   ALREADY claimed by "DNA" or "Chemistry" in `detect_modality` —
        //   a genuine extension conflict, not a missing-feature gap; there
        //   is no unclaimed real proteomics file extension to route on.
        // - Depth: `LiDARFile` needs `format`/`coordinate_system` enums and
        //   3 required bools (`has_intensity`/`has_rgb`/`has_classification`)
        //   that are real facts about the capture, not guessable from the
        //   file alone.
        // - Electromagnetic: `IQFile` needs `sample_rate_hz`/
        //   `center_freq_hz`/`gain_db` — real capture parameters intrinsic
        //   to the recording hardware, impossible to know from the file.
        // Fabricating any of the above would be exactly the kind of false
        // "analyzed" claim this session's no-fabrication doctrine forbids —
        // these 4 correctly fall through to the honest generic fallback.
        _ => None,
    }
}

/// Real ZSEI graph wiring for a resource link (task 64) — previously
/// file/url/package links were entirely flat-JSON, invisible to any real
/// graph traversal or relationship. Creates a real `container_type`
/// container (FileReference here; url_link/package_link mirror this with
/// URLReference/PackageReference) parented under `project_id` when present
/// (falling back to `fallback_root` — FILE_GRAPH_ROOT_ID=76 for files —
/// otherwise; `create_container` degrades gracefully if project_id doesn't
/// correspond to a real container, confirmed by reading
/// text-modality's persist_graph_container and src/zsei/query.rs, so this
/// is safe even for an ad-hoc/nonexistent project_id). Then writes a real
/// bidirectional Relation edge (project Contains reference; reference
/// PartOf project) — best-effort: if the project container can't be
/// fetched (e.g. a genuinely nonexistent project_id), the reference
/// container is still real and created, just not cross-linked, which is
/// still strictly better than the previous fully-flat state.
async fn link_reference_to_graph(
    project_id: u64,
    fallback_root: u64,
    container_type: &str,
    provenance: &str,
    name: String,
    keywords: Vec<String>,
) -> Option<u64> {
    let now = chrono::Utc::now().timestamp() as u64;
    let parent_id = if project_id != 0 { project_id } else { fallback_root };
    let container = serde_json::json!({
        "global_state": { "container_id": 0, "child_count": 0, "version": 1, "parent_id": 0, "child_ids": [] },
        "local_state": {
            "metadata": {
                "container_type": container_type,
                "modality": "Unknown",
                "created_at": now,
                "updated_at": now,
                "provenance": provenance,
                "permissions": 0,
                "owner_id": 0,
                "name": name,
                "materialized_path": null
            },
            "context": {
                "categories": [],
                "methodologies": [],
                "keywords": keywords,
                "topics": [],
                "relationships": [],
                "learned_associations": [],
                "embedding": null
            },
            "storage": { "db_shard_id": null, "vector_index_ref": null, "object_store_path": null, "compression_type": "None" },
            "hints": { "access_frequency": 0, "hotness_score": 0.0, "last_accessed": 0, "centroid": null, "ml_prediction_weight": 0.0 },
            "integrity": { "content_hash": vec![0u8; 32], "semantic_fingerprint": [], "last_verified": now, "integrity_score": 1.0, "version_history": [] },
            "file_context": null,
            "code_context": null,
            "text_context": null,
            "external_ref": null
        }
    });

    let result = zsei_query(serde_json::json!({
        "CreateContainer": { "parent_id": parent_id, "container": container }
    }))
    .await
    .ok()?;
    let container_id = result.get("ContainerID").and_then(|v| v.as_u64())?;

    if project_id != 0 {
        link_relation_both_ways(project_id, container_id).await;
    }

    Some(container_id)
}

/// Best-effort bidirectional Relation edge between a project container and
/// a newly-created reference container — Contains from the project's side,
/// PartOf from the reference's side (RelationType::Contains=3/PartOf=2,
/// src/types/container.rs). Real edges via UpdateContainer, same pattern
/// text/code modality's link_related_containers already proved working.
async fn link_relation_both_ways(project_id: u64, ref_container_id: u64) {
    for (from_id, to_id, relation_type) in [
        (project_id, ref_container_id, "Contains"),
        (ref_container_id, project_id, "PartOf"),
    ] {
        let existing = match zsei_query(serde_json::json!({ "GetContainer": { "container_id": from_id } })).await {
            Ok(v) => v,
            Err(_) => continue,
        };
        let container_json = existing.get("Container").cloned().unwrap_or(existing);
        let context = match container_json.pointer("/local_state/context") {
            Some(c) => c.clone(),
            None => continue,
        };
        let mut relationships: Vec<serde_json::Value> =
            context.get("relationships").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let already = relationships.iter().any(|r| r.get("target_id").and_then(|v| v.as_u64()) == Some(to_id));
        if already {
            continue;
        }
        relationships.push(serde_json::json!({
            "target_id": to_id,
            "relation_type": relation_type,
            "confidence": 1.0,
            "discovered_via": "Manual"
        }));
        let mut updated_context = context.clone();
        updated_context["relationships"] = serde_json::Value::Array(relationships);
        let _ = zsei_query(serde_json::json!({
            "UpdateContainer": {
                "container_id": from_id,
                "updates": { "metadata": null, "context": updated_context, "storage": null, "hints": null }
            }
        }))
        .await;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum FileLinkInput {
    /// Link a single file to a project
    Link {
        project_id: u64,
        file_path: String,
        analyze: bool,
    },
    /// Link multiple files to a project
    LinkMultiple {
        project_id: u64,
        file_paths: Vec<String>,
        analyze: bool,
    },
    /// Unlink a file from a project
    Unlink { project_id: u64, file_ref_id: u64 },
    /// Refresh file status (check if exists, updated, etc.)
    Refresh { file_ref_id: u64 },
    /// Get file reference status
    GetStatus { file_ref_id: u64 },
    /// List all file references for a project
    ListFiles { project_id: u64 },
}

/// File reference info - stored and returned
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRefInfo {
    pub id: u64, // Use 'id' for consistency with UI
    pub project_id: u64,
    pub path: String,
    pub name: String,
    pub exists: bool,
    pub size: u64,
    pub modified: u64,
    pub modality: String,
    pub analyzed: bool,
    pub created_at: u64,
    /// The real ZSEI FileReference container this link was also wired to
    /// (task 64 — previously a link was flat-JSON-only, invisible to any
    /// graph traversal). `#[serde(default)]` so files_<id>.json written
    /// before this field existed still deserialize (as None).
    #[serde(default)]
    pub zsei_container_id: Option<u64>,
    /// The real modality-graph container id produced by actually dispatching
    /// this file to its detected modality's pipeline (e.g. image/audio/video
    /// pipeline #102/103/104's real `Analyze` action) via `POST
    /// /pipeline/execute` on the host. `None` when the modality isn't wired
    /// to a real dispatch yet (falls back to honest metadata-only) or the
    /// real dispatch failed. `#[serde(default)]` so pre-existing records
    /// still deserialize.
    #[serde(default)]
    pub analysis_graph_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLinkOutput {
    pub success: bool,
    pub file_ref: Option<FileRefInfo>,
    pub file_refs: Option<Vec<FileRefInfo>>,
    pub error: Option<String>,
}

// ============================================================================
// Storage Layer
// ============================================================================

fn storage_path() -> PathBuf {
    let base = std::env::var("OZONE_DATA_PATH").unwrap_or_else(|_| "./data".to_string());
    PathBuf::from(base).join("workspaces")
}

fn load_file_refs(project_id: u64) -> Vec<FileRefInfo> {
    let path = storage_path().join(format!("files_{}.json", project_id));
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(refs) = serde_json::from_str(&content) {
                return refs;
            }
        }
    }
    vec![]
}

fn save_file_refs(project_id: u64, refs: &[FileRefInfo]) -> Result<(), String> {
    let dir = storage_path();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create dir: {}", e))?;
    let path = dir.join(format!("files_{}.json", project_id));
    let content =
        serde_json::to_string_pretty(refs).map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to write: {}", e))?;
    Ok(())
}

fn generate_id() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
        % 10_000_000_000
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Detect modality from file extension
fn detect_modality(path: &str) -> String {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Comprehensive extension mapping matching ModalityType
    match ext.as_str() {
        // Code
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "c" | "cpp" | "h" | "hpp" | "java" | "go"
        | "rb" | "php" | "swift" | "kt" | "cs" | "vue" | "svelte" | "scala" | "r" | "jl"
        | "lua" | "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd" | "sql" | "graphql"
        | "elm" | "clj" | "ex" | "exs" | "erl" | "hs" | "ml" | "fs" | "nim" | "zig" | "v" | "d"
        | "ada" | "pas" | "pl" | "pm" | "tcl" | "awk" | "sed" | "makefile" | "cmake"
        | "dockerfile" => "Code",

        // Text/Documentation
        "txt" | "md" | "rst" | "adoc" | "org" => "Text",

        // Data formats (text-based)
        "json" | "yaml" | "yml" | "toml" | "xml" | "csv" => "Data",

        // Config
        "conf" | "cfg" | "ini" | "env" | "gitignore" | "dockerignore" | "editorconfig" => "Config",

        // Web
        "html" | "htm" | "css" | "scss" | "sass" | "less" => "Web",

        // Documents
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "rtf" => "Document",

        // Image
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "ico" | "bmp" | "tiff" | "tif"
        | "heic" | "heif" | "raw" | "cr2" | "nef" | "arw" | "dng" | "psd" | "ai" | "eps" => "Image",

        // Audio
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "wma" | "aiff" | "aif" | "opus"
        | "mid" | "midi" => "Audio",

        // Video
        "mp4" | "mov" | "avi" | "mkv" | "webm" | "wmv" | "flv" | "m4v" | "mpeg" | "mpg" | "3gp"
        | "ogv" | "ts" | "mts" | "m2ts" => "Video",

        // Math
        "tex" | "latex" | "nb" | "m" | "mpl" | "maple" | "mw" | "wxm" | "wxmx" | "sage"
        | "ipynb" | "rmd" | "qmd" => "Math",

        // Chemistry
        "mol" | "sdf" | "pdb" | "xyz" | "cif" | "mol2" | "cml" | "cdx" | "cdxml" | "rxn"
        | "smi" | "smiles" | "inchi" => "Chemistry",

        // DNA/Genomics
        "fasta" | "fa" | "fna" | "ffn" | "faa" | "frn" | "fastq" | "fq" | "gbk" | "gb"
        | "genbank" | "gff" | "gff3" | "gtf" | "vcf" | "sam" | "bam" | "cram" | "bed" | "wig"
        | "bigwig" | "bw" | "2bit" => "DNA",

        // EEG/Neural
        "edf" | "bdf" | "gdf" | "set" | "fif" | "vhdr" | "vmrk" | "eeg" | "cnt" | "avg" | "mff"
        | "ncs" | "nev" | "plx" => "EEG",

        // Archive
        "zip" | "tar" | "gz" | "rar" | "7z" | "bz2" | "xz" => "Archive",

        // 3D (new 2026-10-17 — these extensions previously fell to
        // "Unknown"; none conflict with any bucket above)
        "gltf" | "glb" | "obj" | "fbx" | "stl" | "usd" | "usda" | "usdc" | "usdz" | "3ds"
        | "ply" | "dae" | "blend" => "3D",

        // CAD (new 2026-10-17 — no conflicts with any bucket above)
        "step" | "stp" | "iges" | "igs" | "brep" | "occ" | "f3d" | "fcstd" | "sldprt"
        | "sldasm" => "CAD",

        // Kinematics (new 2026-10-17 — no conflicts with any bucket above)
        "urdf" | "mjcf" | "bvh" | "c3d" | "osim" => "Kinematics",
        // NOTE: "sdf" is Kinematics' SDF (Simulation Description Format) in
        // its own enum, but this extension is ALSO used by chemistry's SDF
        // (structure-data file) — already claimed by "Chemistry" above.
        // Real, pre-existing ambiguity; left with chemistry's prior claim
        // rather than silently reassigning it.

        // Control (new 2026-10-17, deliberately narrow — see
        // `analyze_action_payload`'s long comment on why only Simulink's
        // `.slx` is wired; no conflict with any bucket above)
        "slx" => "Control",

        // Default
        _ => "Unknown",
    }
    .to_string()
}

/// Get file metadata
fn get_file_metadata(path: &str) -> (bool, u64, u64) {
    let path = std::path::Path::new(path);
    if let Ok(meta) = fs::metadata(path) {
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        (true, meta.len(), modified)
    } else {
        (false, 0, 0)
    }
}

/// Create a FileRefInfo from a file path with optional analysis
fn create_file_ref(project_id: u64, file_path: &str, analyze: bool) -> FileRefInfo {
    let (exists, size, modified) = get_file_metadata(file_path);
    let name = std::path::Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let modality = detect_modality(file_path);

    FileRefInfo {
        id: generate_id(),
        project_id,
        path: file_path.to_string(),
        name,
        exists,
        size,
        modified,
        modality,
        analyzed: false, // Will be set true after actual analysis
        created_at: now(),
        // Wired to the real graph at link time (task 64); None here is
        // legitimate — link_reference_to_graph fills it in afterwards.
        zsei_container_id: None,
        analysis_graph_id: None,
    }
}

/// Analyze a file using CodeAnalysis or TextAnalysis based on modality
async fn analyze_file(file_path: &str, modality: &str) -> Result<serde_json::Value, String> {
    // Real dispatch first, for the modalities actually wired to one — these
    // are binary-safe (never read as text) and get a real graph from the
    // real pipeline, not metadata-only guessing. Checked as a `(Some,
    // Some)` pair (not `?`-propagated on a lone `ok_or_else`, fixed
    // 2026-10-17): some wired modalities (Chemistry/CAD/Kinematics) only
    // cover a SUBSET of their own `detect_modality` extensions — e.g.
    // Chemistry's `.xyz`/`.cif` legitimately have no payload builder. Those
    // must fall through to the honest generic fallback below, not error out
    // — a hard `?` here would turn "no real parser for this sub-format"
    // into a failed link for files that used to at least get an honest
    // metadata-only result.
    if let (Some(pipeline_id), Some(payload)) = (
        modality_pipeline_id(modality),
        analyze_action_payload(modality, file_path),
    ) {
        let output = invoke_modality_pipeline(pipeline_id, payload).await?;
        let graph_id = output.get("graph_id").and_then(|g| g.as_u64());
        return Ok(serde_json::json!({
            "analysis_type": "real_pipeline_dispatch",
            "modality": modality,
            "pipeline_id": pipeline_id,
            "graph_id": graph_id,
            "pipeline_output": output
        }));
    }

    // Everything below here is the honest fallback for modalities NOT yet
    // wired to a real dispatch (confirmed this session: Image/Audio/Video/
    // Text/Code are wired above; the remaining modalities — math/chemistry/
    // dna/eeg/etc. — have real CreateContainer-based graph creation by now
    // but aren't wired into this file yet; adding each is the same shape as
    // above, left for a follow-up pass. "Script"/"Web" below have no real
    // modality pipeline to dispatch to at all — not a gap, there's nothing
    // to wire them to).
    match modality {
        // "Code" is no longer handled here — it's intercepted above by the
        // real `modality_pipeline_id`/`analyze_action_payload` dispatch to
        // pipeline 101. "Script"/"Web" have no real modality pipeline in
        // `src/orchestrator/graphs.rs`'s id map (100-126) and remain here,
        // honestly approximate, not a fabricated full-fidelity result.
        "Script" | "Web" => {
            // Read file content — text-safe for these 2 modalities only.
            let content =
                fs::read_to_string(file_path).map_err(|e| format!("Failed to read file: {}", e))?;

            // No real pipeline exists for these two — local structural
            // approximation only, honestly labeled below, not claiming to
            // be the real CodeAnalysis/TextAnalysis result.
            let lines: Vec<&str> = content.lines().collect();
            let functions = lines
                .iter()
                .filter(|l| l.contains("fn ") || l.contains("function ") || l.contains("def "))
                .count();
            let classes = lines
                .iter()
                .filter(|l| l.contains("class ") || l.contains("struct ") || l.contains("impl "))
                .count();
            let imports = lines
                .iter()
                .filter(|l| {
                    l.trim().starts_with("use ")
                        || l.trim().starts_with("import ")
                        || l.trim().starts_with("from ")
                        || l.trim().starts_with("#include")
                })
                .count();

            Ok(serde_json::json!({
                "analysis_type": "code",
                "lines": lines.len(),
                "functions": functions,
                "classes": classes,
                "imports": imports,
                "modality": modality
            }))
        }
        "Documentation" | "Data" => {
            // Read file content — text-safe for these 2 modalities only.
            let content =
                fs::read_to_string(file_path).map_err(|e| format!("Failed to read file: {}", e))?;
            // Call TextAnalysis pipeline (#20) via subprocess
            let words: Vec<&str> = content.split_whitespace().collect();
            let sentences = content
                .chars()
                .filter(|c| *c == '.' || *c == '!' || *c == '?')
                .count();
            let paragraphs = content
                .split("\n\n")
                .filter(|p| !p.trim().is_empty())
                .count();

            Ok(serde_json::json!({
                "analysis_type": "text",
                "words": words.len(),
                "sentences": sentences,
                "paragraphs": paragraphs,
                "modality": modality
            }))
        }
        _ => {
            // Binary or unknown, and not yet wired to a real pipeline above
            // — honest metadata only. `fs::metadata` (not `read_to_string`)
            // so this never fails on real binary content (the bug this
            // whole fix addresses: the old code read every file as text
            // unconditionally, before even reaching this arm).
            let size_bytes = fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);
            Ok(serde_json::json!({
                "analysis_type": "metadata",
                "modality": modality,
                "size_bytes": size_bytes
            }))
        }
    }
}

/// Store analysis result in ZSEI
fn store_analysis(
    project_id: u64,
    file_ref_id: u64,
    analysis: &serde_json::Value,
) -> Result<(), String> {
    let zsei_path = std::env::var("OZONE_ZSEI_PATH").unwrap_or_else(|_| "./zsei_data".to_string());
    let analysis_dir = format!("{}/local/file_analysis", zsei_path);
    fs::create_dir_all(&analysis_dir).ok();

    let analysis_file = format!("{}/{}_{}.json", analysis_dir, project_id, file_ref_id);
    let content = serde_json::to_string_pretty(analysis).map_err(|e| e.to_string())?;
    fs::write(&analysis_file, content).map_err(|e| format!("Failed to write analysis: {}", e))?;

    Ok(())
}

// ============================================================================
// Pipeline Execution
// ============================================================================

pub async fn execute(input: FileLinkInput) -> Result<FileLinkOutput, String> {
    match input {
        FileLinkInput::Link {
            project_id,
            file_path,
            analyze,
        } => {
            let mut refs = load_file_refs(project_id);

            // Check if already linked
            if refs.iter().any(|r| r.path == file_path) {
                return Ok(FileLinkOutput {
                    success: false,
                    file_ref: None,
                    file_refs: None,
                    error: Some("File already linked to this project".to_string()),
                });
            }

            let mut file_ref = create_file_ref(project_id, &file_path, analyze);

            // Perform analysis if requested and file exists
            if analyze && file_ref.exists {
                match analyze_file(&file_path, &file_ref.modality).await {
                    Ok(analysis) => {
                        file_ref.analysis_graph_id =
                            analysis.get("graph_id").and_then(|g| g.as_u64());
                        // Store analysis in ZSEI
                        if store_analysis(project_id, file_ref.id, &analysis).is_ok() {
                            file_ref.analyzed = true;
                        }
                    }
                    Err(e) => {
                        // Log but don't fail - analysis is optional
                        eprintln!("File analysis warning: {}", e);
                    }
                }
            }

            // Real graph wiring (task 64) — a genuine FileReference
            // container + bidirectional Relation edge with the project,
            // not just a flat JSON row. Best-effort: a failure here doesn't
            // fail the link itself (the flat ref is still the source of
            // truth for fast lookup), just leaves zsei_container_id None.
            let keywords: Vec<String> = vec![file_ref.name.to_lowercase(), file_ref.modality.to_lowercase()];
            file_ref.zsei_container_id = link_reference_to_graph(
                project_id,
                76, // FILE_GRAPH_ROOT_ID
                "FileReference",
                "file_link_pipeline",
                format!("File: {}", file_ref.path),
                keywords,
            )
            .await;

            refs.push(file_ref.clone());
            save_file_refs(project_id, &refs)?;

            Ok(FileLinkOutput {
                success: true,
                file_ref: Some(file_ref),
                file_refs: None,
                error: None,
            })
        }

        FileLinkInput::LinkMultiple {
            project_id,
            file_paths,
            analyze,
        } => {
            let mut refs = load_file_refs(project_id);
            let mut new_refs = vec![];

            for file_path in file_paths {
                // Skip if already linked
                if refs.iter().any(|r| r.path == file_path) {
                    continue;
                }

                let mut file_ref = create_file_ref(project_id, &file_path, analyze);

                // Perform analysis if requested and file exists
                if analyze && file_ref.exists {
                    if let Ok(analysis) = analyze_file(&file_path, &file_ref.modality).await {
                        file_ref.analysis_graph_id =
                            analysis.get("graph_id").and_then(|g| g.as_u64());
                        if store_analysis(project_id, file_ref.id, &analysis).is_ok() {
                            file_ref.analyzed = true;
                        }
                    }
                }

                let keywords: Vec<String> = vec![file_ref.name.to_lowercase(), file_ref.modality.to_lowercase()];
                file_ref.zsei_container_id = link_reference_to_graph(
                    project_id,
                    76,
                    "FileReference",
                    "file_link_pipeline",
                    format!("File: {}", file_ref.path),
                    keywords,
                )
                .await;

                refs.push(file_ref.clone());
                new_refs.push(file_ref);
            }

            save_file_refs(project_id, &refs)?;

            Ok(FileLinkOutput {
                success: true,
                file_ref: None,
                file_refs: Some(new_refs),
                error: None,
            })
        }

        FileLinkInput::Unlink {
            project_id,
            file_ref_id,
        } => {
            let mut refs = load_file_refs(project_id);
            let initial_len = refs.len();
            let zsei_container_id = refs.iter().find(|r| r.id == file_ref_id).and_then(|r| r.zsei_container_id);
            refs.retain(|r| r.id != file_ref_id);

            if refs.len() < initial_len {
                save_file_refs(project_id, &refs)?;
                // Best-effort: remove the real graph container too, not
                // just the flat row — an unlinked file shouldn't remain
                // discoverable as if still attached.
                if let Some(container_id) = zsei_container_id {
                    let _ = zsei_query(serde_json::json!({
                        "DeleteContainer": { "container_id": container_id }
                    }))
                    .await;
                }
                Ok(FileLinkOutput {
                    success: true,
                    file_ref: None,
                    file_refs: None,
                    error: None,
                })
            } else {
                Ok(FileLinkOutput {
                    success: false,
                    file_ref: None,
                    file_refs: None,
                    error: Some("File reference not found".to_string()),
                })
            }
        }

        FileLinkInput::Refresh { file_ref_id } => {
            // Find the file ref across all projects (could be optimized with an index)
            let dir = storage_path();
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with("files_") && name.ends_with(".json") {
                            if let Ok(content) = fs::read_to_string(&path) {
                                if let Ok(mut refs) =
                                    serde_json::from_str::<Vec<FileRefInfo>>(&content)
                                {
                                    if let Some(file_ref) =
                                        refs.iter_mut().find(|r| r.id == file_ref_id)
                                    {
                                        // Refresh metadata
                                        let (exists, size, modified) =
                                            get_file_metadata(&file_ref.path);
                                        file_ref.exists = exists;
                                        file_ref.size = size;
                                        file_ref.modified = modified;

                                        // Clone out of the mutable borrow
                                        // before serializing the vec.
                                        let updated = file_ref.clone();

                                        // Save updated refs
                                        let content = serde_json::to_string_pretty(&refs)
                                            .map_err(|e| e.to_string())?;
                                        fs::write(&path, content).map_err(|e| e.to_string())?;

                                        return Ok(FileLinkOutput {
                                            success: true,
                                            file_ref: Some(updated),
                                            file_refs: None,
                                            error: None,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Ok(FileLinkOutput {
                success: false,
                file_ref: None,
                file_refs: None,
                error: Some("File reference not found".to_string()),
            })
        }

        FileLinkInput::GetStatus { file_ref_id } => {
            // Find the file ref
            let dir = storage_path();
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with("files_") && name.ends_with(".json") {
                            if let Ok(content) = fs::read_to_string(&path) {
                                if let Ok(refs) = serde_json::from_str::<Vec<FileRefInfo>>(&content)
                                {
                                    if let Some(file_ref) =
                                        refs.iter().find(|r| r.id == file_ref_id)
                                    {
                                        return Ok(FileLinkOutput {
                                            success: true,
                                            file_ref: Some(file_ref.clone()),
                                            file_refs: None,
                                            error: None,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Ok(FileLinkOutput {
                success: false,
                file_ref: None,
                file_refs: None,
                error: Some("File reference not found".to_string()),
            })
        }

        FileLinkInput::ListFiles { project_id } => {
            let refs = load_file_refs(project_id);
            Ok(FileLinkOutput {
                success: true,
                file_ref: None,
                file_refs: Some(refs),
                error: None,
            })
        }
    }
}

/// Real orchestrator invocations pass the full PipelineInput envelope
/// {data, context} (see RegistryExecutorAdapter::execute) — this used to
/// parse input_json directly as FileLinkInput with no envelope unwrap, so
/// every real host-triggered call would have failed with a missing-field
/// `action` parse error (confirmed by cross-referencing context_aggregation
/// and text/code modality's main(), all of which already unwrap `data`).
/// Also accepts a bare envelope-free FileLinkInput (direct CLI/manual
/// testing) or stdin, matching the established parse_cli_input convention.
fn parse_cli_input<T: serde::de::DeserializeOwned>() -> Result<T, String> {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--input" && i + 1 < args.len() {
            input_json = Some(args[i + 1].clone());
            i += 2;
        } else {
            i += 1;
        }
    }
    let raw = match input_json {
        Some(s) => s,
        None => {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf).map_err(|e| e.to_string())?;
            buf
        }
    };
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let inner = v.get("data").cloned().unwrap_or(v);
    serde_json::from_value(inner).map_err(|e| e.to_string())
}

fn main() {
    let input: FileLinkInput = parse_cli_input().unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        std::process::exit(1);
    });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => {
            println!("{}", serde_json::json!({"success": false, "error": e}));
            std::process::exit(1);
        }
    }
}
