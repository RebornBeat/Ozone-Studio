//! FileLinkPipeline - Pipeline #30
//! Link local files to projects. Files are referenced, not copied.
//!
//! STORAGE: Persists file references to JSON files in the data directory
//! Called FROM WorkspaceTab UI - does NOT need its own tab

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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
    }
}

/// Analyze a file using CodeAnalysis or TextAnalysis based on modality
fn analyze_file(file_path: &str, modality: &str) -> Result<serde_json::Value, String> {
    // Read file content
    let content =
        fs::read_to_string(file_path).map_err(|e| format!("Failed to read file: {}", e))?;

    match modality {
        "Code" | "Script" | "Web" => {
            // Call CodeAnalysis pipeline (#18) via subprocess
            // In real implementation, this would call the pipeline executor
            let input = serde_json::json!({
                "action": "Analyze",
                "code": content,
                "file_path": file_path,
                "use_llm": false // Structural analysis only
            });

            // For now, perform basic structural analysis inline
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
            // Binary or unknown - basic metadata only
            Ok(serde_json::json!({
                "analysis_type": "metadata",
                "modality": modality,
                "size_bytes": content.len()
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
                match analyze_file(&file_path, &file_ref.modality) {
                    Ok(analysis) => {
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
                    if let Ok(analysis) = analyze_file(&file_path, &file_ref.modality) {
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
