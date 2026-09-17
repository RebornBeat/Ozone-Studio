//! PackageLinkPipeline - Pipeline #32
//! Link external packages to projects. API context extracted, not copied.
//!
//! STORAGE: Persists package references to JSON files in the data directory
//! — was previously a complete no-op stub (Link fabricated a random id and
//! stored nothing at all; ScanProject always returned a hardcoded fake
//! react@18.0.0 regardless of the real project). Real persistence, real
//! ZSEI graph wiring (task 64), and a real (honest, non-fabricated) scan
//! built 2026-09-16.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum PackageLinkInput {
    Link { project_id: u64, registry: String, name: String, version: Option<String> },
    LinkMultiple { project_id: u64, packages: Vec<PackageSpec> },
    Unlink { project_id: u64, package_ref_id: u64 },
    Refresh { package_ref_id: u64 },
    GetStatus { package_ref_id: u64 },
    ScanProject { project_id: u64 }, // Auto-detect packages from linked manifest files
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSpec { pub registry: String, pub name: String, pub version: Option<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageRefInfo {
    pub package_ref_id: u64,
    pub project_id: u64,
    pub registry: String,
    pub name: String,
    pub version: String,
    pub context_extracted: bool,
    pub last_updated: u64,
    /// The real ZSEI PackageReference container this link was also wired
    /// to (task 64). `#[serde(default)]` so files predating this field
    /// still deserialize (as None).
    #[serde(default)]
    pub zsei_container_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageLinkOutput {
    pub success: bool,
    pub package_ref_id: Option<u64>,
    pub package_ref_ids: Option<Vec<u64>>,
    pub package_info: Option<PackageRefInfo>,
    pub package_infos: Option<Vec<PackageRefInfo>>,
    pub detected_packages: Option<Vec<PackageSpec>>,
    pub error: Option<String>,
}

// ============================================================================
// Storage layer — same real, flat-JSON-per-project convention as
// file_link/url_link (packages_<project_id>.json under workspaces/).
// ============================================================================

fn storage_path() -> PathBuf {
    let base = std::env::var("OZONE_DATA_PATH").unwrap_or_else(|_| "./data".to_string());
    PathBuf::from(base).join("workspaces")
}

fn load_package_refs(project_id: u64) -> Vec<PackageRefInfo> {
    let path = storage_path().join(format!("packages_{}.json", project_id));
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(refs) = serde_json::from_str(&content) {
                return refs;
            }
        }
    }
    vec![]
}

fn save_package_refs(project_id: u64, refs: &[PackageRefInfo]) -> Result<(), String> {
    let dir = storage_path();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create dir: {}", e))?;
    let path = dir.join(format!("packages_{}.json", project_id));
    let content = serde_json::to_string_pretty(refs).map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to write: {}", e))?;
    Ok(())
}

fn generate_id() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64 % 10_000_000_000
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}

// ============================================================================
// Real ZSEI graph wiring (task 64) — same pattern as file_link/url_link.
// ============================================================================

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
        return Err(body.get("error").and_then(|e| e.as_str()).unwrap_or("zsei query failed").to_string());
    }
    Ok(body.get("result").cloned().unwrap_or(serde_json::Value::Null))
}

/// root=70 (EXTERNAL_PACKAGES_ROOT_ID) is the fallback parent.
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

// ============================================================================
// Real (non-fabricated) manifest scanning — reads files_<project_id>.json,
// the real storage file_link's own pipeline already writes, and parses any
// linked manifest by its real content. No filesystem-layout guessing: if
// nothing manifest-like is linked, returns an honest empty result rather
// than a fabricated package (the previous behavior, unconditionally
// "react@18.0.0" regardless of the real project).
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
struct LinkedFileRef {
    path: String,
    #[serde(default)]
    exists: bool,
}

fn linked_file_paths(project_id: u64) -> Vec<String> {
    let path = storage_path().join(format!("files_{}.json", project_id));
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let refs: Vec<LinkedFileRef> = serde_json::from_str(&content).unwrap_or_default();
    refs.into_iter().filter(|r| r.exists).map(|r| r.path).collect()
}

fn parse_package_json(content: &str) -> Vec<PackageSpec> {
    let mut out = Vec::new();
    let parsed: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return out,
    };
    for key in ["dependencies", "devDependencies"] {
        if let Some(deps) = parsed.get(key).and_then(|v| v.as_object()) {
            for (name, version) in deps {
                out.push(PackageSpec {
                    registry: "npm".to_string(),
                    name: name.clone(),
                    version: version.as_str().map(|s| s.trim_start_matches(['^', '~']).to_string()),
                });
            }
        }
    }
    out
}

fn parse_cargo_toml(content: &str) -> Vec<PackageSpec> {
    let mut out = Vec::new();
    let mut in_deps_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps_section = trimmed == "[dependencies]" || trimmed == "[dev-dependencies]";
            continue;
        }
        if !in_deps_section || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((name, rest)) = trimmed.split_once('=') {
            let name = name.trim().to_string();
            // Handles `name = "1.2.3"` and `name = { version = "1.2.3", ... }`
            // — a best-effort real parse of the common cases, not a full
            // TOML parser (no new dependency added for this one field).
            let version = rest
                .split("version")
                .nth(1)
                .and_then(|s| s.split('"').nth(1))
                .or_else(|| rest.trim().trim_matches('"').split('"').next())
                .map(|s| s.trim_matches('"').to_string())
                .filter(|s| !s.is_empty() && !s.starts_with('{'));
            out.push(PackageSpec { registry: "crates.io".to_string(), name, version });
        }
    }
    out
}

fn parse_requirements_txt(content: &str) -> Vec<PackageSpec> {
    content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (name, version) = if let Some((n, v)) = l.split_once("==") {
                (n.trim().to_string(), Some(v.trim().to_string()))
            } else {
                (l.trim().to_string(), None)
            };
            PackageSpec { registry: "pypi".to_string(), name, version }
        })
        .collect()
}

fn parse_go_mod(content: &str) -> Vec<PackageSpec> {
    let mut out = Vec::new();
    let mut in_require_block = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("require (") {
            in_require_block = true;
            continue;
        }
        if in_require_block && trimmed == ")" {
            in_require_block = false;
            continue;
        }
        let entry = if in_require_block {
            Some(trimmed)
        } else {
            trimmed.strip_prefix("require ")
        };
        if let Some(entry) = entry {
            let mut parts = entry.split_whitespace();
            if let (Some(name), Some(version)) = (parts.next(), parts.next()) {
                out.push(PackageSpec {
                    registry: "go".to_string(),
                    name: name.to_string(),
                    version: Some(version.to_string()),
                });
            }
        }
    }
    out
}

fn scan_linked_manifests(project_id: u64) -> Vec<PackageSpec> {
    let mut detected = Vec::new();
    for path in linked_file_paths(project_id) {
        let file_name = std::path::Path::new(&path).file_name().and_then(|n| n.to_str()).unwrap_or("");
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let mut found = match file_name {
            "package.json" => parse_package_json(&content),
            "Cargo.toml" => parse_cargo_toml(&content),
            "requirements.txt" => parse_requirements_txt(&content),
            "go.mod" => parse_go_mod(&content),
            _ => continue,
        };
        detected.append(&mut found);
    }
    detected
}

// ============================================================================
// Pipeline Execution
// ============================================================================

pub async fn execute(input: PackageLinkInput) -> Result<PackageLinkOutput, String> {
    match input {
        PackageLinkInput::Link { project_id, registry, name, version } => {
            let mut refs = load_package_refs(project_id);
            if refs.iter().any(|r| r.registry == registry && r.name == name) {
                return Ok(PackageLinkOutput {
                    success: false,
                    package_ref_id: None,
                    package_ref_ids: None,
                    package_info: None,
                    package_infos: None,
                    detected_packages: None,
                    error: Some("Package already linked to this project".to_string()),
                });
            }
            let version_str = version.unwrap_or_else(|| "unknown".to_string());
            let mut info = PackageRefInfo {
                package_ref_id: generate_id(),
                project_id,
                registry: registry.clone(),
                name: name.clone(),
                version: version_str,
                context_extracted: false,
                last_updated: now(),
                zsei_container_id: None,
            };
            info.zsei_container_id = link_reference_to_graph(
                project_id,
                70, // EXTERNAL_PACKAGES_ROOT_ID
                "PackageReference",
                "package_link_pipeline",
                format!("Package: {}/{}", registry, name),
                vec![name.to_lowercase(), registry.to_lowercase()],
            )
            .await;
            refs.push(info.clone());
            save_package_refs(project_id, &refs)?;
            Ok(PackageLinkOutput {
                success: true,
                package_ref_id: Some(info.package_ref_id),
                package_ref_ids: None,
                package_info: Some(info),
                package_infos: None,
                detected_packages: None,
                error: None,
            })
        }

        PackageLinkInput::LinkMultiple { project_id, packages } => {
            let mut refs = load_package_refs(project_id);
            let mut new_infos = vec![];
            for spec in packages {
                if refs.iter().any(|r| r.registry == spec.registry && r.name == spec.name) {
                    continue;
                }
                let mut info = PackageRefInfo {
                    package_ref_id: generate_id(),
                    project_id,
                    registry: spec.registry.clone(),
                    name: spec.name.clone(),
                    version: spec.version.clone().unwrap_or_else(|| "unknown".to_string()),
                    context_extracted: false,
                    last_updated: now(),
                    zsei_container_id: None,
                };
                info.zsei_container_id = link_reference_to_graph(
                    project_id,
                    70,
                    "PackageReference",
                    "package_link_pipeline",
                    format!("Package: {}/{}", spec.registry, spec.name),
                    vec![spec.name.to_lowercase(), spec.registry.to_lowercase()],
                )
                .await;
                refs.push(info.clone());
                new_infos.push(info);
            }
            save_package_refs(project_id, &refs)?;
            let ids = new_infos.iter().map(|i| i.package_ref_id).collect();
            Ok(PackageLinkOutput {
                success: true,
                package_ref_id: None,
                package_ref_ids: Some(ids),
                package_info: None,
                package_infos: Some(new_infos),
                detected_packages: None,
                error: None,
            })
        }

        PackageLinkInput::Unlink { project_id, package_ref_id } => {
            let mut refs = load_package_refs(project_id);
            let initial_len = refs.len();
            let zsei_container_id =
                refs.iter().find(|r| r.package_ref_id == package_ref_id).and_then(|r| r.zsei_container_id);
            refs.retain(|r| r.package_ref_id != package_ref_id);
            if refs.len() < initial_len {
                save_package_refs(project_id, &refs)?;
                if let Some(container_id) = zsei_container_id {
                    let _ = zsei_query(serde_json::json!({ "DeleteContainer": { "container_id": container_id } })).await;
                }
                Ok(PackageLinkOutput {
                    success: true,
                    package_ref_id: None,
                    package_ref_ids: None,
                    package_info: None,
                    package_infos: None,
                    detected_packages: None,
                    error: None,
                })
            } else {
                Ok(PackageLinkOutput {
                    success: false,
                    package_ref_id: None,
                    package_ref_ids: None,
                    package_info: None,
                    package_infos: None,
                    detected_packages: None,
                    error: Some("Package reference not found".to_string()),
                })
            }
        }

        PackageLinkInput::Refresh { package_ref_id } => {
            let dir = storage_path();
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with("packages_") && name.ends_with(".json") {
                            if let Ok(content) = fs::read_to_string(&path) {
                                if let Ok(mut refs) = serde_json::from_str::<Vec<PackageRefInfo>>(&content) {
                                    if let Some(info) = refs.iter_mut().find(|r| r.package_ref_id == package_ref_id) {
                                        info.last_updated = now();
                                        // Clone out of the mutable borrow
                                        // before serializing the vec.
                                        let updated = info.clone();
                                        let content = serde_json::to_string_pretty(&refs).map_err(|e| e.to_string())?;
                                        fs::write(&path, content).map_err(|e| e.to_string())?;
                                        return Ok(PackageLinkOutput {
                                            success: true,
                                            package_ref_id: None,
                                            package_ref_ids: None,
                                            package_info: Some(updated),
                                            package_infos: None,
                                            detected_packages: None,
                                            error: None,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(PackageLinkOutput {
                success: false,
                package_ref_id: None,
                package_ref_ids: None,
                package_info: None,
                package_infos: None,
                detected_packages: None,
                error: Some("Package reference not found".to_string()),
            })
        }

        PackageLinkInput::GetStatus { package_ref_id } => {
            let dir = storage_path();
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with("packages_") && name.ends_with(".json") {
                            if let Ok(content) = fs::read_to_string(&path) {
                                if let Ok(refs) = serde_json::from_str::<Vec<PackageRefInfo>>(&content) {
                                    if let Some(info) = refs.iter().find(|r| r.package_ref_id == package_ref_id) {
                                        return Ok(PackageLinkOutput {
                                            success: true,
                                            package_ref_id: None,
                                            package_ref_ids: None,
                                            package_info: Some(info.clone()),
                                            package_infos: None,
                                            detected_packages: None,
                                            error: None,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(PackageLinkOutput {
                success: false,
                package_ref_id: None,
                package_ref_ids: None,
                package_info: None,
                package_infos: None,
                detected_packages: None,
                error: Some("Package reference not found".to_string()),
            })
        }

        PackageLinkInput::ScanProject { project_id } => {
            // Real, honest scan of whatever manifest files are actually
            // linked to this project (via file_link's own real storage) —
            // never a fabricated result regardless of what's really there.
            let detected = scan_linked_manifests(project_id);
            Ok(PackageLinkOutput {
                success: true,
                package_ref_id: None,
                package_ref_ids: None,
                package_info: None,
                package_infos: None,
                detected_packages: Some(detected),
                error: None,
            })
        }
    }
}

/// See file_link's identical helper for the full rationale — real
/// orchestrator calls pass the {data, context} envelope.
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
    let input: PackageLinkInput = parse_cli_input().unwrap_or_else(|e| {
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

#[cfg(test)]
mod tests {
    use super::*;

    // Real manifest parsing — the previous ScanProject fabricated
    // react@18.0.0 unconditionally; these prove real content actually
    // drives the result now.

    #[test]
    fn package_json_dependencies_and_dev_dependencies_are_parsed() {
        let content = r#"{"dependencies": {"react": "^18.2.0"}, "devDependencies": {"vite": "~5.0.0"}}"#;
        let out = parse_package_json(content);
        assert_eq!(out.len(), 2);
        assert!(out.iter().any(|p| p.name == "react" && p.version.as_deref() == Some("18.2.0") && p.registry == "npm"));
        assert!(out.iter().any(|p| p.name == "vite" && p.version.as_deref() == Some("5.0.0")));
    }

    #[test]
    fn package_json_with_no_deps_is_a_real_empty_result_not_fabricated() {
        let out = parse_package_json(r#"{"name": "x"}"#);
        assert!(out.is_empty());
    }

    #[test]
    fn cargo_toml_dependencies_section_is_parsed() {
        let content = "[package]\nname = \"foo\"\n\n[dependencies]\nserde = \"1.0\"\ntokio = { version = \"1.35\", features = [\"full\"] }\n\n[dev-dependencies]\nmockall = \"0.12\"\n";
        let out = parse_cargo_toml(content);
        assert!(out.iter().any(|p| p.name == "serde" && p.version.as_deref() == Some("1.0")));
        assert!(out.iter().any(|p| p.name == "tokio" && p.version.as_deref() == Some("1.35")));
        assert!(out.iter().any(|p| p.name == "mockall"));
        // [package] section's `name = "foo"` must NOT be picked up as a dependency.
        assert!(!out.iter().any(|p| p.name == "name"));
    }

    #[test]
    fn requirements_txt_pinned_and_unpinned_lines_are_parsed() {
        let content = "requests==2.31.0\nflask\n# a comment\n\nnumpy==1.26.0\n";
        let out = parse_requirements_txt(content);
        assert_eq!(out.len(), 3);
        assert!(out.iter().any(|p| p.name == "requests" && p.version.as_deref() == Some("2.31.0")));
        assert!(out.iter().any(|p| p.name == "flask" && p.version.is_none()));
    }

    #[test]
    fn go_mod_require_block_is_parsed() {
        let content = "module example.com/foo\n\nrequire (\n\tgithub.com/gorilla/mux v1.8.0\n\tgithub.com/lib/pq v1.10.9\n)\n\nrequire github.com/other/pkg v0.1.0\n";
        let out = parse_go_mod(content);
        assert_eq!(out.len(), 3);
        assert!(out.iter().any(|p| p.name == "github.com/gorilla/mux" && p.version.as_deref() == Some("v1.8.0")));
        assert!(out.iter().any(|p| p.name == "github.com/other/pkg"));
    }
}
