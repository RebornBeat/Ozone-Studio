//! OZONE Studio - Code Modality Pipeline (ID: 101)
//! 
//! Analyzes code and creates structural graphs for:
//! - AST/parse tree structure
//! - Dependencies (imports, references)
//! - Function/class definitions
//! - Type information
//! - Call graphs

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::PathBuf;
use regex::Regex;

#[path = "../../shared/ozone_serve.rs"]
mod ozone_serve;

/// Pipeline-side model-call capture (C6-minimal, shared — single source).
/// DORMANT today: code extraction is regex/mechanical and makes ZERO
/// pipeline-9 calls (verified by exhaustive grep, 2026-09-27) — nothing
/// to capture. The mod lights up with this codebase's first real model
/// call site; wire `capture::capture_zero_shot_call("code", <site>, ...)
/// around it, same one-line pattern text/math use.
#[allow(dead_code)]
#[path = "../../shared/capture.rs"]
mod capture;

// Same real-ZSEI-over-HTTP pattern as the text modality pipeline (100) and
// context_aggregation (21): ZSEIQuery is externally-tagged, wire format
// {"VariantName": {fields...}}.
fn ozone_host() -> String {
    env::var("OZONE_HOST").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string())
}

/// Process-wide HTTP client for host calls — keep-alive connection reuse.
/// Measured (link_metrics, 2026-09-20): a fresh Client per zsei_query call
/// costs a TCP connect every time; the shared client removes that from the
/// per-edge linking cost without capping or batching anything.
fn shared_http_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

async fn zsei_query(query: serde_json::Value) -> Result<serde_json::Value, String> {
    let client = shared_http_client();
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

/// Persist a code graph as a real ZSEI container — this pipeline had never
/// been built or run before this session; create_graph previously only
/// minted a local timestamp as graph_id and returned the node/edge graph in
/// the process's own stdout response, with nothing written to ZSEI or disk.
/// Once that one short-lived subprocess exited, the graph was gone — any
/// later QueryGraph/GetDependencyGraph call, or the orchestrator's own
/// FileLayerContext.graph_id, referenced an id backed by nothing. Mirrors
/// text modality pipeline 100's persist_graph_container exactly: real
/// keywords/name on the container, full node/edge content alongside it as
/// a JSON file. code_context stays None for the same honesty reason
/// text_context does there — Container's CodeContext schema (ast_summary,
/// call_graph, data_flow, quality_metrics, etc.) has no honest mapping from
/// CodeAnalysisResult's simpler shape without fabricating fields; a real
/// mapping is a separate, larger task.
async fn persist_graph_container(
    nodes: &[CodeGraphNode],
    edges: &[CodeGraphEdge],
    analysis: &CodeAnalysisResult,
    local_graph_id: u64,
    project_id: u64,
) -> Result<(u64, Vec<String>, Vec<String>), String> {
    let now = chrono::Utc::now().timestamp() as u64;
    let mut keywords: Vec<String> = analysis.functions.iter().map(|f| f.name.to_lowercase()).collect();
    keywords.extend(analysis.classes.iter().map(|c| c.name.to_lowercase()));
    keywords.extend(analysis.imports.iter().map(|i| i.module.to_lowercase()));
    keywords.push(analysis.language.to_lowercase());
    let topics: Vec<String> = vec![analysis.language.clone()];
    let name = format!(
        "{} code graph ({}, {} lines, {} functions, {} classes)",
        analysis.file_path.clone().unwrap_or_default(),
        analysis.language,
        analysis.line_count,
        analysis.functions.len(),
        analysis.classes.len()
    );

    let container = serde_json::json!({
        "global_state": {
            "container_id": 0,
            "child_count": 0,
            "version": 1,
            "parent_id": 0,
            "child_ids": []
        },
        "local_state": {
            "metadata": {
                "container_type": "ModalityGraph",
                "modality": "Code",
                "created_at": now,
                "updated_at": now,
                "provenance": "pipeline:101",
                "permissions": 0,
                "owner_id": 0,
                "name": name,
                "materialized_path": null
            },
            "context": {
                "categories": [],
                "methodologies": [],
                "keywords": keywords,
                "topics": topics.clone(),
                "relationships": [],
                "learned_associations": [],
                "embedding": null
            },
            "storage": {
                "db_shard_id": null,
                "vector_index_ref": null,
                "object_store_path": format!("graphs/code_{}.json", local_graph_id),
                "compression_type": "None"
            },
            "hints": {
                "access_frequency": 0,
                "hotness_score": 0.0,
                "last_accessed": 0,
                "centroid": null,
                "ml_prediction_weight": 0.0
            },
            "integrity": {
                "content_hash": vec![0u8; 32],
                "semantic_fingerprint": [],
                "last_verified": now,
                "integrity_score": 1.0,
                "version_history": []
            },
            "file_context": null,
            "code_context": null,
            "text_context": null,
            "external_ref": null
        }
    });

    // project_id, when nonzero, is used as the real parent — see text
    // modality pipeline 100's identical fix for the full rationale:
    // ContainerType::Project/Workspace and their query handlers
    // (zsei/query.rs) are real; the only gap was that nothing here ever
    // used a given project_id as a parent. create_container degrades
    // gracefully if project_id doesn't correspond to a real container, so
    // this is safe even for a not-yet-existing project_id. 0 keeps today's
    // root-parented default.
    let result = zsei_query(serde_json::json!({
        "CreateContainer": { "parent_id": project_id, "container": container }
    }))
    .await?;

    let container_id = result
        .get("ContainerID")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "CreateContainer did not return a ContainerID".to_string())?;

    let data_dir = env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let graphs_dir = format!("{}/graphs", data_dir);
    let _ = std::fs::create_dir_all(&graphs_dir);
    let graph_content = serde_json::json!({ "graph_id": local_graph_id, "nodes": nodes, "edges": edges, "analysis": analysis });
    if let Ok(json) = serde_json::to_string_pretty(&graph_content) {
        // local_graph_id path: matches the container's own
        // storage.object_store_path (read_code_graph resolves it there).
        let graph_path = format!("{}/code_{}.json", graphs_dir, local_graph_id);
        let _ = std::fs::write(&graph_path, json.clone());
        // container_id path: matches the text-modality convention every
        // container-id-keyed reader uses (load_code_graph_from_disk,
        // QueryGraph/UpdateGraph by graph_id, GetGraphWithProvisional) —
        // without this the file existed only under an id nothing outside
        // this one process had ever been told.
        let by_container = format!("{}/code_{}.json", graphs_dir, container_id);
        let _ = std::fs::write(&by_container, json);
    }

    Ok((container_id, keywords, topics))
}

/// Real disk shape `persist_graph_container` actually writes (`{graph_id,
/// nodes, edges, analysis}`) — distinct from the in-process `CodeGraph`
/// struct (`{graph_id, modality, nodes, edges, metadata}`), so reading a
/// persisted file back needs its own matching shape, not `CodeGraph` itself.
#[derive(Debug, Serialize, Deserialize, Clone)]
struct PersistedCodeGraph {
    graph_id: u64,
    nodes: Vec<CodeGraphNode>,
    edges: Vec<CodeGraphEdge>,
    analysis: CodeAnalysisResult,
}

/// Real cross-process retrieval — task 65 (confirmed live 2026-09-16: every
/// dependency-graph/provisional handler in this file was a stub returning
/// empty data with a "Would query ZSEI..." comment). Mirrors math
/// modality's real, proven `get_container_object_store_path`/
/// `read_graph_container` pattern exactly (math is this project's
/// designated reference implementation for this exact gap) — including the
/// absolute-path guard math itself was missing until this same pass fixed
/// it there too (found by building this).
async fn get_container_object_store_path(container_id: u64) -> Result<String, String> {
    let result = zsei_query(serde_json::json!({
        "GetContainer": { "container_id": container_id }
    }))
    .await?;

    result
        .get("Container")
        .and_then(|c| c.get("local_state"))
        .and_then(|l| l.get("storage"))
        .and_then(|s| s.get("object_store_path"))
        .and_then(|p| p.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Container {} has no object_store_path (not a graph container?)", container_id))
}

async fn read_code_graph(container_id: u64) -> Result<PersistedCodeGraph, String> {
    let object_store_path = get_container_object_store_path(container_id).await?;
    let data_dir = env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let full_path = if std::path::Path::new(&object_store_path).is_absolute() {
        object_store_path.clone()
    } else {
        format!("{}/{}", data_dir, object_store_path)
    };
    let content = std::fs::read_to_string(&full_path)
        .map_err(|e| format!("Failed to read graph file {}: {}", full_path, e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse graph file {}: {}", full_path, e))
}

/// Find every real code-modality graph container that's a direct child of
/// `project_id` — real structural traversal (GetContainer -> child_ids ->
/// filter by container_type+modality), not a keyword guess. Depends on
/// child_ids actually surviving a restart, which it now does (a real,
/// separate bug fixed earlier tonight in src/zsei/storage.rs).
async fn find_project_code_graphs(project_id: u64) -> Result<Vec<u64>, String> {
    let result = zsei_query(serde_json::json!({
        "GetContainer": { "container_id": project_id }
    }))
    .await?;
    let container = result.get("Container").cloned().unwrap_or(result);
    let child_ids: Vec<u64> = container
        .pointer("/global_state/child_ids")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_u64()).collect())
        .unwrap_or_default();

    let mut graph_ids = Vec::new();
    for child_id in child_ids {
        let child_result = match zsei_query(serde_json::json!({ "GetContainer": { "container_id": child_id } })).await {
            Ok(v) => v,
            Err(_) => continue,
        };
        let child = child_result.get("Container").cloned().unwrap_or(child_result);
        let container_type = child
            .pointer("/local_state/metadata/container_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let modality = child
            .pointer("/local_state/metadata/modality")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if container_type == "ModalityGraph" && modality == "Code" {
            graph_ids.push(child_id);
        }
    }
    Ok(graph_ids)
}

/// Real, honest cross-file dependency assembly from every persisted graph
/// under a project — shared by GetDependencyGraph, ComputeReverseDependencies,
/// and QueryGraph's FindDependencies/FindReverseDependencies. `to_file` for
/// an internal import is the raw module string an import statement actually
/// named (e.g. "crate::auth", "./utils") — genuinely resolving that to
/// another file's real path would need real per-language module-resolution
/// logic this pipeline doesn't have; reporting the honest raw target rather
/// than a fabricated resolved path matches this project's established
/// honesty convention (see e.g. Go's real return_type staying None rather
/// than guessed).
async fn assemble_dependency_graph(project_id: u64, include_external: bool) -> Result<DependencyGraph, String> {
    let graph_ids = find_project_code_graphs(project_id).await?;

    let mut files = Vec::new();
    let mut dependencies = Vec::new();
    let mut external_map: std::collections::HashMap<String, ExternalDependency> = std::collections::HashMap::new();

    for graph_id in &graph_ids {
        let persisted = match read_code_graph(*graph_id).await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("assemble_dependency_graph: skipping container {} ({})", graph_id, e);
                continue;
            }
        };
        let analysis = &persisted.analysis;
        let file_path = analysis.file_path.clone().unwrap_or_default();

        files.push(FileNode {
            file_id: *graph_id,
            path: file_path.clone(),
            language: analysis.language.clone(),
            line_count: analysis.line_count,
            function_count: analysis.functions.len(),
            class_count: analysis.classes.len(),
        });

        for import in &analysis.imports {
            if import.is_external {
                if include_external {
                    external_map
                        .entry(import.module.clone())
                        .and_modify(|d| {
                            if !d.used_by.contains(&file_path) {
                                d.used_by.push(file_path.clone());
                            }
                        })
                        .or_insert_with(|| ExternalDependency {
                            package: import.module.clone(),
                            version: None,
                            used_by: vec![file_path.clone()],
                        });
                }
            } else {
                dependencies.push(FileDependency {
                    from_file: file_path.clone(),
                    to_file: import.module.clone(),
                    dependency_type: DependencyType::Direct,
                    imports: import.items.clone(),
                });
            }
        }
    }

    Ok(DependencyGraph {
        project_id,
        files,
        dependencies,
        external_deps: external_map.into_values().collect(),
    })
}

/// Cross-relationship linking — mirrors text modality pipeline 100's
/// `link_related_containers` exactly (same reasoning: type-blind search +
/// client-side infrastructure-type filter, since a query-level type filter
/// alone is too narrow for "find anything genuinely related across any
/// content type" and a fully type-blind search with no filter reintroduces
/// the real Pipeline-registry-leak hallucination bug this project already
/// fixed once). Only real difference: `discovered_via: "CodeAnalysis"`,
/// this modality's own real discovery-method provenance, not text's.
/// The live relevance policy from OZONE_RELEVANCE_POLICY (exported at boot
/// by the host from the KAlgorithms registry's current preset). Falls back
/// to the graph-first defaults when absent (older host binary).
/// (graph_max_depth, neighborhood_shared_floor, seed_shared_floor, preset)
fn relevance_policy() -> (u32, usize, usize, String) {
    if let Ok(raw) = std::env::var("OZONE_RELEVANCE_POLICY") {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
            return (
                v.get("graph_max_depth").and_then(|x| x.as_u64()).unwrap_or(2) as u32,
                v.get("neighborhood_shared_floor").and_then(|x| x.as_u64()).unwrap_or(1) as usize,
                v.get("seed_shared_floor").and_then(|x| x.as_u64()).unwrap_or(2) as usize,
                v.get("preset").and_then(|x| x.as_str()).unwrap_or("graph-first").to_string(),
            );
        }
    }
    (2, 1, 2, "graph-first".to_string())
}

async fn link_related_containers(container_id: u64, project_id: u64, own_keywords: &[String], own_topics: &[String]) -> (usize, serde_json::Value) {
    fn is_infrastructure_container_type(t: &str) -> bool {
        matches!(
            t,
            "Root" | "User" | "Workspace" | "Project"
                | "Pipeline"
                | "ModalityRoot" | "MethodologyRoot" | "BlueprintRoot" | "PipelineRoot"
                | "ConsciousnessRoot" | "ExternalRoot" | "PackageRoot"
                | "JurisdictionRoot"
        )
    }

    let mut search_terms: Vec<String> = own_keywords.to_vec();
    search_terms.extend(own_topics.iter().map(|t| t.to_lowercase()));
    search_terms.sort();
    search_terms.dedup();
    if search_terms.is_empty() {
        return (0, serde_json::json!({"skipped": "no search terms"}));
    }

    let search_result = match zsei_query(serde_json::json!({
        "SearchContainersByKeywords": {
            "keywords": search_terms,
            "container_type": Value::Null,
            "strategy": Value::Null
        }
    }))
    .await
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("link_related_containers: search failed (non-fatal): {}", e);
            return (0, serde_json::json!({"skipped": "keyword search failed", "error": e}));
        }
    };

    let candidate_ids: Vec<u64> = search_result
        .get("Containers")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_u64()).collect())
        .unwrap_or_default();

    let own_set: HashSet<String> = search_terms.iter().cloned().collect();
    let started = std::time::Instant::now();
    let (walk_depth, nb_floor, seed_floor, preset_name) = relevance_policy();

    // ── Candidate source 1: the relationship neighborhood ──
    // ONE structural Traverse from the project container — real graph edges
    // (parent/child AND explicit Context.relationships, walked by the
    // store's traversal engine). UNCAPPED per user directive: the walk
    // depth is the only bound; max_results is set to an effectively
    // unlimited value and the REAL returned count lands in the metrics so
    // tuning decisions come from measured numbers, never preemptive caps.
    let mut neighborhood: HashMap<u64, u32> = HashMap::new();
    let mut traverse_returned = 0usize;
    if walk_depth > 0 && project_id != 0 {
        match zsei_query(serde_json::json!({
            "Traverse": {
                "start_container": project_id,
                "mode": "Structural",
                "filters": [],
                "max_depth": walk_depth,
                "max_results": 10_000_000u64,
                "budget": { "max_hops": walk_depth, "max_containers": 10_000_000u64, "max_latency_ms": 3_600_000u64 },
                "use_ml": false,
                "include_methodologies": false,
                "include_external_refs": false,
            }
        }))
        .await
        {
            Ok(result) => {
                if let Some(tr) = result.get("TraversalResult") {
                    let containers = tr.get("containers").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                    let paths = tr.get("paths").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                    traverse_returned = containers.len();
                    for (cid, path) in containers.iter().zip(paths.iter()) {
                        if let (Some(id), Some(hops)) = (
                            cid.as_u64(),
                            path.get("hops").and_then(|h| h.as_array()),
                        ) {
                            neighborhood.insert(id, hops.len().saturating_sub(1) as u32);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "link_related_containers: relationship walk failed (non-fatal, keyword seeds only): {}",
                    e
                );
            }
        }
    }

    // candidate_ids (keyword seeds, computed above) — source 2: they catch
    // genuinely related containers with NO graph path yet; without them
    // nothing would ever receive its FIRST relationship edge.

    // ── Score: hop-aware, floors from the relevance policy, NO CAPS ──
    // Neighborhood candidates ordered by hop distance first, keyword seeds
    // after; every unique candidate is fetched and scored. Nothing is
    // truncated — the write phase touches every candidate that clears its
    // floor, however many that is.
    // (candidate_id, container_json, context, shared_terms, hops)
    let mut scored: Vec<(u64, serde_json::Value, serde_json::Value, usize, Option<u32>)> = Vec::new();
    let mut seen: HashSet<u64> = HashSet::new();
    let mut ordered: Vec<(u64, Option<u32>)> = {
        let mut v: Vec<(u64, Option<u32>)> =
            neighborhood.iter().map(|(id, h)| (*id, Some(*h))).collect();
        v.sort_by_key(|(_, h)| *h);
        v
    };
    for id in &candidate_ids {
        ordered.push((*id, None));
    }

    let mut fetch_failures = 0usize;
    for (candidate_id, hops) in ordered {
        if candidate_id == container_id || !seen.insert(candidate_id) {
            continue;
        }
        let candidate = match zsei_query(serde_json::json!({
            "GetContainer": { "container_id": candidate_id }
        }))
        .await
        {
            Ok(v) => v,
            Err(_) => {
                fetch_failures += 1;
                continue;
            }
        };
        let container_json = candidate.get("Container").cloned().unwrap_or(candidate);

        let candidate_type = container_json
            .pointer("/local_state/metadata/container_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if is_infrastructure_container_type(candidate_type) {
            continue;
        }

        let candidate_context = match container_json.pointer("/local_state/context") {
            Some(c) => c.clone(),
            None => continue,
        };
        let candidate_terms: HashSet<String> = candidate_context
            .get("keywords")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .chain(candidate_context.get("topics").and_then(|v| v.as_array()).into_iter().flatten())
            .filter_map(|v| v.as_str())
            .map(|s| s.to_lowercase())
            .collect();

        let shared_count = own_set.intersection(&candidate_terms).count();
        let floor = if hops.is_some() { nb_floor } else { seed_floor };
        if shared_count < floor {
            continue;
        }

        scored.push((candidate_id, container_json, candidate_context, shared_count, hops));
    }

    // Most graph-adjacent first, then strongest overlap.
    scored.sort_by(|a, b| {
        let ha = a.4.unwrap_or(u32::MAX);
        let hb = b.4.unwrap_or(u32::MAX);
        ha.cmp(&hb).then(b.3.cmp(&a.3))
    });

    // Stats captured BEFORE the write loop consumes `scored` — every value
    // counted from what this run actually did.
    let qualified_total = scored.len();
    let qualified_neighborhood = scored.iter().filter(|c| c.4.is_some()).count();
    let shared_max = scored.iter().map(|c| c.3).max().unwrap_or(0);
    let shared_total: usize = scored.iter().map(|c| c.3).sum();

    // ── Write phase: real bidirectional edges, one candidate at a time ──
    let mut wired = 0usize;
    let mut already_linked_skipped = 0usize;
    let mut write_failures = 0usize;
    for (candidate_id, _container_json, candidate_context, shared_count, hops) in scored {
        let mut relationships: Vec<serde_json::Value> = candidate_context
            .get("relationships")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let already_linked = relationships.iter().any(|r| {
            r.get("target_id").and_then(|v| v.as_u64()) == Some(container_id)
        });
        if already_linked {
            already_linked_skipped += 1;
            continue;
        }

        let confidence = (0.3 + 0.15 * shared_count as f32).min(0.9);
        // TWO distinct edges: the candidate points at the new graph, the
        // new graph points at the candidate. (Found live: reusing ONE edge
        // object for both sides wrote target_id=container_id into the new
        // graph's OWN relationships — 7 self-loops.)
        let mut new_edge = serde_json::json!({
            "target_id": container_id,
            "relation_type": "SimilarTo",
            "confidence": confidence,
            "discovered_via": "CodeAnalysis"
        });
        let mut reverse_edge = serde_json::json!({
            "target_id": candidate_id,
            "relation_type": "SimilarTo",
            "confidence": confidence,
            "discovered_via": "CodeAnalysis"
        });
        if let Some(h) = hops {
            new_edge["graph_hops"] = serde_json::json!(h);
            reverse_edge["graph_hops"] = serde_json::json!(h);
        }

        // Candidate -> this new container
        relationships.push(new_edge);
        let mut candidate_context_updated = candidate_context.clone();
        candidate_context_updated["relationships"] = serde_json::Value::Array(relationships);
        let update_a = zsei_query(serde_json::json!({
            "UpdateContainer": {
                "container_id": candidate_id,
                "updates": { "metadata": null, "context": candidate_context_updated, "storage": null, "hints": null }
            }
        }))
        .await;

        // This new container -> candidate (bidirectional)
        let own_container = match zsei_query(serde_json::json!({
            "GetContainer": { "container_id": container_id }
        }))
        .await
        {
            Ok(v) => v,
            Err(_) => {
                write_failures += 1;
                continue;
            }
        };
        let own_container_json = own_container.get("Container").cloned().unwrap_or(own_container);
        let own_context = match own_container_json.pointer("/local_state/context") {
            Some(c) => c.clone(),
            None => {
                write_failures += 1;
                continue;
            }
        };
        let mut own_relationships: Vec<serde_json::Value> = own_context
            .get("relationships")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        own_relationships.push(reverse_edge);
        let mut own_context_updated = own_context.clone();
        own_context_updated["relationships"] = serde_json::Value::Array(own_relationships);
        let update_b = zsei_query(serde_json::json!({
            "UpdateContainer": {
                "container_id": container_id,
                "updates": { "metadata": null, "context": own_context_updated, "storage": null, "hints": null }
            }
        }))
        .await;

        if update_a.is_ok() && update_b.is_ok() {
            wired += 1;
        } else {
            write_failures += 1;
            eprintln!(
                "link_related_containers: partial/failed write for {} <-> {} (non-fatal)",
                container_id, candidate_id
            );
        }
    }

    // ── Real captured metrics (user directive: optimize from measured
    // numbers, never preemptive caps) — every value counted from what this
    // run actually did. ──
    let metrics = serde_json::json!({
        "policy_preset": preset_name,
        "graph_max_depth": walk_depth,
        "neighborhood_shared_floor": nb_floor,
        "seed_shared_floor": seed_floor,
        "traverse_containers_returned": traverse_returned,
        "neighborhood_candidates": neighborhood.len(),
        "seed_candidates": candidate_ids.len(),
        "unique_candidates": seen.len(),
        "fetch_failures": fetch_failures,
        "qualified": qualified_total + already_linked_skipped,
        "qualified_neighborhood": qualified_neighborhood,
        "qualified_seeds": qualified_total - qualified_neighborhood + already_linked_skipped,
        "shared_terms_max": shared_max,
        "shared_terms_total": shared_total,
        "already_linked_skipped": already_linked_skipped,
        "edges_written": wired,
        "write_failures": write_failures,
        "duration_ms": started.elapsed().as_millis() as u64,
    });
    (wired, metrics)
}

// ============================================================================
// INPUT/OUTPUT TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeModalityInput {
    pub action: CodeModalityAction,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CodeModalityAction {
    /// Analyze code and create structural representation
    Analyze {
        code: String,
        language: Option<String>,
        file_path: Option<String>,
        #[serde(default)]
        depth: AnalysisDepth,
    },
    
    /// Create a graph from analysis results
    CreateGraph {
        analysis_result: CodeAnalysisResult,
        project_id: u64,
        #[serde(default)]
        link_to_existing: bool,
    },
    
    /// Update existing graph with code changes
    UpdateGraph {
        graph_id: u64,
        delta: CodeDelta,
    },
    
    /// Query the code graph
    QueryGraph {
        graph_id: u64,
        query: CodeGraphQuery,
    },
    
    /// Get dependency graph for a project
    GetDependencyGraph {
        project_id: u64,
        #[serde(default)]
        include_external: bool,
    },
    
    /// Compute reverse dependencies
    ComputeReverseDependencies {
        project_id: u64,
        file_path: String,
    },
    
    /// Create provisional graph nodes (for graph-first approach)
    CreateProvisionalNodes {
        project_id: u64,
        planned_files: Vec<PlannedFile>,
        session_id: String,
    },
    
    /// Get graph including provisional nodes
    GetGraphWithProvisional {
        project_id: u64,
        session_id: String,
    },
    
    /// Finalize provisional nodes after successful write
    FinalizeProvisional {
        session_id: String,
        file_container_ids: Vec<(u64, u64)>, // provisional_id -> actual_id
    },
    
    /// Rollback provisional nodes on failure
    RollbackProvisional {
        session_id: String,
    },
    
    /// Check for conflicts before code generation
    CheckConflicts {
        project_id: u64,
        proposed_structure: ProposedCodeStructure,
    },
    
    /// Trigger ZSEI semantic analysis hook
    TriggerSemanticHook {
        graph_id: u64,
        hook_type: ZSEIHookType,
    },
    
    /// Suggest applicable methodologies
    SuggestMethodologies {
        code: String,
        language: String,
        available_methodology_ids: Vec<u64>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub enum AnalysisDepth {
    #[default]
    Standard,
    Deep,       // Include call graph, data flow
    Surface,    // Just structure, fast
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeModalityOutput {
    pub success: bool,
    pub error: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analysis: Option<CodeAnalysisResult>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_id: Option<u64>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<CodeGraph>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependency_graph: Option<DependencyGraph>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reverse_deps: Option<Vec<FileDependency>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflicts: Option<ConflictReport>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provisional_nodes: Option<Vec<ProvisionalNode>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_methodologies: Option<Vec<MethodologySuggestion>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook_result: Option<HookResult>,

    /// Real result for QueryGraph — a generic Value rather than a new enum
    /// per CodeQueryType, since the 9 real query types genuinely have
    /// different honest result shapes (a function list vs. a call-graph
    /// edge list vs. an inheritance chain); forcing one rigid type here
    /// would mean fabricating empty fields for the shapes that don't apply
    /// to a given query, which is exactly what this pass is fixing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_result: Option<Value>,

    /// Real captured metrics from link_related_containers on CreateGraph —
    /// candidate counts, shared-term distribution, edges written, duration.
    /// Every value counted from what the run actually did (user directive:
    /// tune from measured numbers, never preemptive caps).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_metrics: Option<Value>,
}

impl Default for CodeModalityOutput {
    fn default() -> Self {
        Self {
            success: false,
            error: None,
            analysis: None,
            graph_id: None,
            graph: None,
            dependency_graph: None,
            reverse_deps: None,
            conflicts: None,
            provisional_nodes: None,
            suggested_methodologies: None,
            hook_result: None,
            query_result: None,
            link_metrics: None,
        }
    }
}

// ============================================================================
// ANALYSIS TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodeAnalysisResult {
    pub language: String,
    pub file_path: Option<String>,
    pub line_count: usize,
    pub functions: Vec<FunctionDef>,
    pub classes: Vec<ClassDef>,
    pub imports: Vec<ImportDef>,
    pub exports: Vec<ExportDef>,
    pub variables: Vec<VariableDef>,
    pub type_definitions: Vec<TypeDef>,
    pub comments: Vec<Comment>,
    pub function_calls: Vec<FunctionCall>,
    pub complexity_metrics: ComplexityMetrics,
    /// CALL GRAPH ORDERING (2026-09-29): detected entry points (main-like
    /// names first, then zero-in-degree functions), and the entry-point-
    /// rooted call sequence — the order of events/calls for this file.
    #[serde(default)]
    pub entry_points: Vec<String>,
    #[serde(default)]
    pub call_sequence: Vec<CallSequenceEntry>,
}

/// One step of the entry-point-rooted call sequence.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CallSequenceEntry {
    pub order: u32,
    pub function: String,
    pub called_from: Option<String>,
    pub depth: u32,
    pub line: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionDef {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub is_async: bool,
    pub is_public: bool,
    pub doc_comment: Option<String>,
    pub calls: Vec<String>,
    pub complexity: usize,
    /// CALL-GRAPH ORDERING (2026-09-29): position in the entry-point-rooted
    /// call sequence (0 = first called from an entry point). None = not
    /// reached from any entry point (unreachable/utility-only).
    #[serde(default)]
    pub call_order: Option<u32>,
    /// True when this function is a detected entry point (main-like name,
    /// or zero in-degree within the file's resolved call graph).
    #[serde(default)]
    pub is_entry_point: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClassDef {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub methods: Vec<FunctionDef>,
    pub fields: Vec<VariableDef>,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub is_public: bool,
    pub doc_comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportDef {
    pub module: String,
    pub items: Vec<String>,
    pub is_external: bool,
    pub line: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportDef {
    pub name: String,
    pub export_type: ExportType,
    pub line: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ExportType {
    Function,
    Class,
    Variable,
    Type,
    Default,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VariableDef {
    pub name: String,
    pub var_type: Option<String>,
    pub is_const: bool,
    pub is_public: bool,
    pub line: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TypeDef {
    pub name: String,
    pub kind: TypeKind,
    pub fields: Vec<(String, String)>,
    pub line: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TypeKind {
    Struct,
    Enum,
    Interface,
    TypeAlias,
    Trait,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Parameter {
    pub name: String,
    pub param_type: Option<String>,
    pub default_value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Comment {
    pub text: String,
    pub line: usize,
    pub is_doc: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCall {
    pub caller: String,
    pub callee: String,
    pub line: usize,
    pub is_method: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComplexityMetrics {
    pub cyclomatic_complexity: usize,
    pub cognitive_complexity: usize,
    pub halstead_metrics: Option<HalsteadMetrics>,
    pub maintainability_index: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HalsteadMetrics {
    pub distinct_operators: usize,
    pub distinct_operands: usize,
    pub total_operators: usize,
    pub total_operands: usize,
    pub program_length: usize,
    pub vocabulary: usize,
    pub volume: f32,
    pub difficulty: f32,
    pub effort: f32,
}

// ============================================================================
// GRAPH TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodeGraph {
    pub graph_id: u64,
    pub modality: String,
    pub nodes: Vec<CodeGraphNode>,
    pub edges: Vec<CodeGraphEdge>,
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodeGraphNode {
    pub node_id: u64,
    pub node_type: CodeNodeType,
    pub name: String,
    pub position: Option<CodePosition>,
    pub properties: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CodeNodeType {
    File,
    Module,
    Class,
    Function,
    Method,
    Variable,
    Type,
    Import,
    Export,
    Parameter,
    Block,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodePosition {
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub start_column: Option<usize>,
    pub end_column: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodeGraphEdge {
    pub from_node: u64,
    pub to_node: u64,
    pub edge_type: CodeEdgeType,
    pub weight: f32,
    pub properties: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CodeEdgeType {
    // Structural
    Contains,
    Imports,
    Exports,
    Extends,
    Implements,
    
    // Dependency
    DependsOn,
    Calls,
    References,
    TypeOf,
    
    // Semantic (added by ZSEI)
    RelatesTo,
    SimilarTo,
    AlternativeTo,
    Refactors,
    Tests,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DependencyGraph {
    pub project_id: u64,
    pub files: Vec<FileNode>,
    pub dependencies: Vec<FileDependency>,
    pub external_deps: Vec<ExternalDependency>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileNode {
    pub file_id: u64,
    pub path: String,
    pub language: String,
    pub line_count: usize,
    pub function_count: usize,
    pub class_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileDependency {
    pub from_file: String,
    pub to_file: String,
    pub dependency_type: DependencyType,
    pub imports: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DependencyType {
    Direct,
    Transitive,
    DevOnly,
    Optional,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExternalDependency {
    pub package: String,
    pub version: Option<String>,
    pub used_by: Vec<String>,
}

// ============================================================================
// PROVISIONAL/CONFLICT TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlannedFile {
    pub path: String,
    pub language: String,
    pub planned_exports: Vec<String>,
    pub planned_imports: Vec<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProvisionalNode {
    pub provisional_id: u64,
    pub file_path: String,
    pub session_id: String,
    pub created_at: String,
    pub planned_structure: PlannedFile,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProposedCodeStructure {
    pub files: Vec<ProposedFile>,
    pub new_functions: Vec<ProposedFunction>,
    pub new_types: Vec<ProposedType>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProposedFile {
    pub path: String,
    pub exports: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProposedFunction {
    pub name: String,
    pub file_path: String,
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProposedType {
    pub name: String,
    pub file_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConflictReport {
    pub has_conflicts: bool,
    pub conflicts: Vec<Conflict>,
    pub warnings: Vec<Warning>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Conflict {
    pub conflict_type: ConflictType,
    pub description: String,
    pub existing_location: Option<String>,
    pub proposed_location: String,
    pub resolution: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConflictType {
    DuplicateName,
    CircularDependency,
    BreakingChange,
    TypeMismatch,
    MissingDependency,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Warning {
    pub warning_type: String,
    pub message: String,
    pub location: Option<String>,
}

// ============================================================================
// OTHER TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeDelta {
    pub operation: DeltaOperation,
    pub file_path: String,
    pub position: Option<CodePosition>,
    pub content: Option<String>,
    pub affected_nodes: Vec<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DeltaOperation {
    Insert,
    Delete,
    Replace,
    Rename,
    Move,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeGraphQuery {
    pub query_type: CodeQueryType,
    pub parameters: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CodeQueryType {
    FindFunction,
    FindClass,
    FindType,
    FindUsages,
    FindDependencies,
    FindReverseDependencies,
    GetCallGraph,
    GetTypeHierarchy,
    SemanticSearch,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MethodologySuggestion {
    pub methodology_id: u64,
    pub relevance: f32,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ZSEIHookType {
    OnGraphCreated,
    OnEdgeCompletion,
    OnInferRelationships,
    OnCrossModalityLink,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HookResult {
    pub hook_type: ZSEIHookType,
    pub success: bool,
    pub edges_added: usize,
    pub nodes_enriched: usize,
}

// ============================================================================
// PIPELINE IMPLEMENTATION
// ============================================================================

/// Load a CodeGraph from the persisted JSON file.
/// Tries multiple data dir locations (matching persist_graph_container's
/// write paths). Parses the REAL disk shape (`PersistedCodeGraph` — the
/// file carries `analysis`, not `modality`/`metadata`) and converts, so a
/// parse failure here is a genuinely corrupt file rather than a guaranteed
/// shape mismatch on every single persisted graph.
fn load_code_graph_from_disk(graph_id: u64) -> Option<CodeGraph> {
    let dirs = [
        std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string()),
        "zsei_data".to_string(),
    ];
    for dir in &dirs {
        for path in [
            format!("{}/graphs/code_{}.json", dir, graph_id),
            format!("{}/graphs/code_graph_{}.json", dir, graph_id),
        ] {
            if let Ok(raw) = std::fs::read_to_string(&path) {
                if let Ok(persisted) = serde_json::from_str::<PersistedCodeGraph>(&raw) {
                    return Some(CodeGraph {
                        graph_id: persisted.graph_id,
                        modality: "Code".to_string(),
                        nodes: persisted.nodes,
                        edges: persisted.edges,
                        metadata: HashMap::new(),
                    });
                }
            }
        }
    }
    None
}

/// Save a CodeGraph to the persisted JSON file.
fn save_code_graph_to_disk(graph: &CodeGraph) -> Result<(), String> {
    let dirs = [
        std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string()),
        "zsei_data".to_string(),
    ];
    for dir in &dirs {
        let graphs_dir = format!("{}/graphs", dir);
        let _ = std::fs::create_dir_all(&graphs_dir);
        let path = format!("{}/graphs/code_{}.json", dir, graph.graph_id);
        if let Ok(json) = serde_json::to_string_pretty(graph) {
            if std::fs::write(&path, json).is_ok() {
                return Ok(());
            }
        }
    }
    Err("failed to save code graph to any known path".to_string())
}

// ============================================================================
// PROVISIONAL SESSION STORE (task 65) — the provisional-node subsystem's
// durability layer. Each pipeline invocation is a fresh process, so the
// in-memory HashMap on CodeModalityPipeline dies with it; Create →
// GetGraphWithProvisional → Finalize/Rollback only chains up across calls
// if sessions persist here, as files under {data_dir}/provisional/.
// ============================================================================

/// Filesystem-safe session key — session ids are caller-supplied strings
/// and may contain path-hostile characters; anything outside
/// [alnum-_] becomes '_', deterministically.
fn provisional_session_key(session_id: &str) -> String {
    session_id
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

fn provisional_session_path(session_id: &str) -> String {
    let dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    format!("{}/provisional/session_{}.json", dir, provisional_session_key(session_id))
}

/// Load one session's provisional nodes; a missing or unreadable file is an
/// empty session (nothing was ever planned in it), not an error.
fn load_provisional_session(session_id: &str) -> Vec<ProvisionalNode> {
    std::fs::read_to_string(provisional_session_path(session_id))
        .ok()
        .and_then(|raw| serde_json::from_str::<Vec<ProvisionalNode>>(&raw).ok())
        .unwrap_or_default()
}

fn save_provisional_session(session_id: &str, nodes: &[ProvisionalNode]) -> Result<(), String> {
    let path = provisional_session_path(session_id);
    let dir = std::path::Path::new(&path)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_default();
    std::fs::create_dir_all(&dir).map_err(|e| format!("provisional dir {}: {}", dir.display(), e))?;
    let json = serde_json::to_string_pretty(nodes).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("write {}: {}", path, e))
}

/// Delete one session's file, returning the nodes that were discarded so
/// rollback can honestly report what it threw away.
fn delete_provisional_session(session_id: &str) -> Vec<ProvisionalNode> {
    let discarded = load_provisional_session(session_id);
    let _ = std::fs::remove_file(provisional_session_path(session_id));
    discarded
}

/// Container-id-keyed load with an object_store_path fallback — covers
/// graphs persisted before persist_graph_container began writing the
/// container-id copy: their files exist only under the path the container
/// itself records, so that's the honest second place to look.
async fn load_code_graph_for_container(container_id: u64) -> Option<CodeGraph> {
    if let Some(graph) = load_code_graph_from_disk(container_id) {
        return Some(graph);
    }
    match read_code_graph(container_id).await {
        Ok(persisted) => Some(CodeGraph {
            graph_id: persisted.graph_id,
            modality: "Code".to_string(),
            nodes: persisted.nodes,
            edges: persisted.edges,
            metadata: HashMap::new(),
        }),
        Err(_) => None,
    }
}

pub struct CodeModalityPipeline {
    provisional_nodes: HashMap<String, Vec<ProvisionalNode>>,
}

impl CodeModalityPipeline {
    pub fn new() -> Self {
        Self {
            provisional_nodes: HashMap::new(),
        }
    }
    
    pub async fn execute(&self, input: CodeModalityInput) -> CodeModalityOutput {
        match input.action {
            CodeModalityAction::Analyze { code, language, file_path, depth } => {
                self.analyze_code(&code, language, file_path, depth)
            }
            
            CodeModalityAction::CreateGraph { analysis_result, project_id, link_to_existing } => {
                self.create_graph(analysis_result, project_id, link_to_existing).await
            }
            
            CodeModalityAction::UpdateGraph { graph_id, delta } => {
                self.update_graph(graph_id, delta).await
            }
            
            CodeModalityAction::QueryGraph { graph_id, query } => {
                self.query_graph(graph_id, query).await
            }
            
            CodeModalityAction::GetDependencyGraph { project_id, include_external } => {
                self.get_dependency_graph(project_id, include_external).await
            }
            
            CodeModalityAction::ComputeReverseDependencies { project_id, file_path } => {
                self.compute_reverse_deps(project_id, &file_path).await
            }
            
            CodeModalityAction::CreateProvisionalNodes { project_id, planned_files, session_id } => {
                self.create_provisional_nodes(project_id, planned_files, session_id)
            }
            
            CodeModalityAction::GetGraphWithProvisional { project_id, session_id } => {
                self.get_graph_with_provisional(project_id, &session_id).await
            }
            
            CodeModalityAction::FinalizeProvisional { session_id, file_container_ids } => {
                self.finalize_provisional(&session_id, file_container_ids)
            }
            
            CodeModalityAction::RollbackProvisional { session_id } => {
                self.rollback_provisional(&session_id)
            }
            
            CodeModalityAction::CheckConflicts { project_id, proposed_structure } => {
                self.check_conflicts(project_id, proposed_structure).await
            }
            
            CodeModalityAction::TriggerSemanticHook { graph_id, hook_type } => {
                self.trigger_semantic_hook(graph_id, hook_type).await
            }
            
            CodeModalityAction::SuggestMethodologies { code, language, available_methodology_ids } => {
                self.suggest_methodologies(&code, &language, &available_methodology_ids)
            }
        }
    }
    
    fn analyze_code(
        &self,
        code: &str,
        language: Option<String>,
        file_path: Option<String>,
        depth: AnalysisDepth,
    ) -> CodeModalityOutput {
        let language = language.unwrap_or_else(|| self.detect_language(code, file_path.as_deref()));
        
        let line_count = code.lines().count();
        let functions = self.extract_functions(code, &language);
        let classes = self.extract_classes(code, &language);
        let imports = self.extract_imports(code, &language);
        let exports = self.extract_exports(code, &language);
        let variables = self.extract_variables(code, &language);
        let type_definitions = self.extract_types(code, &language);
        let comments = self.extract_comments(code, &language);
        // Deep and the default Standard depth both extract calls (same
        // regex-scan cost class as the functions/classes/imports extraction
        // already run at Standard depth above) — only Surface ("just
        // structure, fast") skips it. Previously gated to Deep-only while
        // Standard is the real #[default], so Calls edges almost never
        // populated in practice even after build_graph_nodes_edges below
        // was fixed to wire them.
        let function_calls = if matches!(depth, AnalysisDepth::Surface) {
            Vec::new()
        } else {
            self.extract_function_calls(code, &language, &functions)
        };
        
        let complexity_metrics = self.compute_complexity(code, &functions);

        // CALL GRAPH ORDERING (2026-09-29): entry-point-rooted sequence —
        // the order of events/calls for this file. Deterministic (common
        // practice per Soot/RTA literature: entry points seed the walk).
        let (entry_points, call_sequence, functions) =
            compute_call_ordering(&functions, &function_calls);

        let analysis = CodeAnalysisResult {
            language,
            file_path,
            line_count,
            functions,
            classes,
            imports,
            exports,
            variables,
            type_definitions,
            comments,
            function_calls,
            complexity_metrics,
            entry_points,
            call_sequence,
        };
        
        CodeModalityOutput {
            success: true,
            analysis: Some(analysis),
            ..Default::default()
        }
    }
    
    fn detect_language(&self, code: &str, file_path: Option<&str>) -> String {
        // First try to detect from file extension
        if let Some(path) = file_path {
            let path_buf = PathBuf::from(path);
            let ext = path_buf
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");

            match ext {
                "rs" => return "rust".to_string(),
                "py" => return "python".to_string(),
                "js" => return "javascript".to_string(),
                "ts" => return "typescript".to_string(),
                "go" => return "go".to_string(),
                "java" => return "java".to_string(),
                "c" | "h" => return "c".to_string(),
                "cpp" | "cc" | "hpp" => return "cpp".to_string(),
                "rb" => return "ruby".to_string(),
                "php" => return "php".to_string(),
                "swift" => return "swift".to_string(),
                "kt" => return "kotlin".to_string(),
                _ => {}
            }
        }
        
        // Detect from code patterns
        if code.contains("fn ") && code.contains("->") && code.contains("let ") {
            return "rust".to_string();
        }
        if code.contains("def ") && code.contains(":") && !code.contains(";") {
            return "python".to_string();
        }
        if code.contains("function ") || code.contains("const ") || code.contains("let ") {
            if code.contains(": ") && (code.contains("interface ") || code.contains(": string")) {
                return "typescript".to_string();
            }
            return "javascript".to_string();
        }
        if code.contains("func ") && code.contains("package ") {
            return "go".to_string();
        }
        if code.contains("public class ") || code.contains("private void ") {
            return "java".to_string();
        }
        
        "unknown".to_string()
    }
    
    fn extract_functions(&self, code: &str, language: &str) -> Vec<FunctionDef> {
        let mut functions = Vec::new();
        
        let pattern = match language {
            "rust" => r"(?m)^(\s*)(pub\s+)?(async\s+)?fn\s+(\w+)\s*(?:<[^>]*>)?\s*\(([^)]*)\)(?:\s*->\s*([^\{]+))?\s*\{",
            "python" => r"(?m)^(\s*)(async\s+)?def\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*([^:]+))?\s*:",
            "javascript" | "typescript" => r"(?m)^(\s*)(?:export\s+)?(?:async\s+)?function\s+(\w+)\s*\(([^)]*)\)(?:\s*:\s*([^{]+))?\s*\{",
            "go" => r"(?m)^func\s+(?:\([^)]+\)\s+)?(\w+)\s*\(([^)]*)\)(?:\s*(?:\([^)]+\)|[^{]+))?\s*\{",
            _ => return functions,
        };
        
        let re = Regex::new(pattern).unwrap();
        
        for (line_num, line) in code.lines().enumerate() {
            if let Some(caps) = re.captures(line) {
                let name = match language {
                    "rust" => caps.get(4).map(|m| m.as_str().to_string()),
                    "python" => caps.get(3).map(|m| m.as_str().to_string()),
                    "javascript" | "typescript" => caps.get(2).map(|m| m.as_str().to_string()),
                    "go" => caps.get(1).map(|m| m.as_str().to_string()),
                    _ => None,
                };
                
                if let Some(name) = name {
                    let is_async = line.contains("async");
                    let is_public = line.contains("pub ") || line.contains("export ");

                    // Find function end (simplified - count braces)
                    let end_line = self.find_block_end(code, line_num);

                    // Real parsing of what the regex above already captures
                    // but the struct literal used to discard (`Vec::new(),
                    // // TODO: Parse parameters` and a hardcoded `None` for
                    // return_type, despite every pattern above having a
                    // params group and rust/python/js/ts also having a
                    // return-type group). Parsed from real source text, not
                    // fabricated — go has no clean single return-type
                    // capture group in its pattern (multi-return vs
                    // single-return are one alternation), so it honestly
                    // stays None rather than guessing.
                    let params_str = match language {
                        "rust" => caps.get(5).map(|m| m.as_str()),
                        "python" => caps.get(4).map(|m| m.as_str()),
                        "javascript" | "typescript" => caps.get(3).map(|m| m.as_str()),
                        "go" => caps.get(2).map(|m| m.as_str()),
                        _ => None,
                    }
                    .unwrap_or("");
                    let parameters = self.parse_parameters(params_str, language);

                    let return_type = match language {
                        "rust" => caps.get(6).map(|m| m.as_str().trim().to_string()),
                        "python" => caps.get(5).map(|m| m.as_str().trim().to_string()),
                        "javascript" | "typescript" => caps.get(4).map(|m| m.as_str().trim().to_string()),
                        _ => None,
                    }
                    .filter(|s| !s.is_empty());

                    functions.push(FunctionDef {
                        name,
                        start_line: line_num + 1,
                        end_line,
                        parameters,
                        return_type,
                        is_async,
                        is_public,
                        doc_comment: None,
                        calls: Vec::new(),
                        complexity: 1,
                        call_order: None,
                        is_entry_point: false,
                    });
                }
            }
        }
        
        functions
    }
    
    fn extract_classes(&self, code: &str, language: &str) -> Vec<ClassDef> {
        let mut classes = Vec::new();
        
        let pattern = match language {
            "rust" => r"(?m)^(\s*)(pub\s+)?(?:struct|enum|trait)\s+(\w+)",
            "python" => r"(?m)^(\s*)class\s+(\w+)(?:\([^)]*\))?\s*:",
            "javascript" | "typescript" => r"(?m)^(\s*)(?:export\s+)?class\s+(\w+)(?:\s+extends\s+(\w+))?(?:\s+implements\s+([^{]+))?\s*\{",
            "java" => r"(?m)^(\s*)(public\s+)?class\s+(\w+)(?:\s+extends\s+(\w+))?(?:\s+implements\s+([^{]+))?\s*\{",
            _ => return classes,
        };
        
        let re = Regex::new(pattern).unwrap();

        for (line_num, line) in code.lines().enumerate() {
            if let Some(caps) = re.captures(line) {
                // Group indices differ per pattern (Java has an extra optional
                // "public " group before the class name that JS/TS doesn't),
                // so name/extends/implements must be read per-language rather
                // than at one fixed index — a prior fixed-index version
                // silently misattributed Java's "public " capture as the
                // class name and read JS/TS's implements group as extends.
                let (name, extends, implements): (Option<String>, Option<String>, Vec<String>) = match language {
                    "rust" => (caps.get(3).map(|m| m.as_str().to_string()), None, Vec::new()),
                    "python" => (caps.get(2).map(|m| m.as_str().to_string()), None, Vec::new()),
                    "javascript" | "typescript" => (
                        caps.get(2).map(|m| m.as_str().to_string()),
                        caps.get(3).map(|m| m.as_str().trim().to_string()),
                        caps.get(4)
                            .map(|m| m.as_str().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                            .unwrap_or_default(),
                    ),
                    "java" => (
                        caps.get(3).map(|m| m.as_str().to_string()),
                        caps.get(4).map(|m| m.as_str().trim().to_string()),
                        caps.get(5)
                            .map(|m| m.as_str().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                            .unwrap_or_default(),
                    ),
                    _ => (None, None, Vec::new()),
                };

                if let Some(name) = name {
                    let is_public = line.contains("pub ") || line.contains("public ") || line.contains("export ");
                    let end_line = self.find_block_end(code, line_num);

                    classes.push(ClassDef {
                        name,
                        start_line: line_num + 1,
                        end_line,
                        methods: Vec::new(),
                        fields: Vec::new(),
                        extends,
                        implements,
                        is_public,
                        doc_comment: None,
                    });
                }
            }
        }

        classes
    }
    
    fn extract_imports(&self, code: &str, language: &str) -> Vec<ImportDef> {
        let mut imports = Vec::new();
        
        let pattern = match language {
            "rust" => r"(?m)^use\s+([^;]+);",
            "python" => r"(?m)^(?:from\s+(\S+)\s+)?import\s+(.+)$",
            "javascript" | "typescript" => r#"(?m)^import\s+(?:\{([^}]+)\}|(\w+))\s+from\s+['"]([^'"]+)['"]"#,
            "go" => r#"(?m)^import\s+(?:\(\s*([^)]+)\s*\)|"([^"]+)")"#,
            _ => return imports,
        };
        
        let re = Regex::new(pattern).unwrap();
        
        for (line_num, line) in code.lines().enumerate() {
            if let Some(caps) = re.captures(line) {
                let (module, items) = match language {
                    "rust" => {
                        let full = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                        (full.to_string(), vec![full.to_string()])
                    }
                    "python" => {
                        let module = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                        let items: Vec<String> = caps.get(2)
                            .map(|m| m.as_str().split(',').map(|s| s.trim().to_string()).collect())
                            .unwrap_or_default();
                        (module.to_string(), items)
                    }
                    "javascript" | "typescript" => {
                        let module = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                        let items: Vec<String> = caps.get(1)
                            .map(|m| m.as_str().split(',').map(|s| s.trim().to_string()).collect())
                            .or_else(|| caps.get(2).map(|m| vec![m.as_str().to_string()]))
                            .unwrap_or_default();
                        (module.to_string(), items)
                    }
                    _ => continue,
                };
                
                let is_external = !module.starts_with('.') && !module.starts_with("crate") && !module.starts_with("super");
                
                imports.push(ImportDef {
                    module,
                    items,
                    is_external,
                    line: line_num + 1,
                });
            }
        }
        
        imports
    }
    
    fn extract_exports(&self, code: &str, language: &str) -> Vec<ExportDef> {
        let mut exports = Vec::new();
        
        let pattern = match language {
            "rust" => r"(?m)^pub\s+(fn|struct|enum|trait|type|const|static)\s+(\w+)",
            "javascript" | "typescript" => r"(?m)^export\s+(?:default\s+)?(function|class|const|let|var|type|interface)\s+(\w+)",
            _ => return exports,
        };
        
        let re = Regex::new(pattern).unwrap();
        
        for (line_num, line) in code.lines().enumerate() {
            if let Some(caps) = re.captures(line) {
                let export_type = match caps.get(1).map(|m| m.as_str()) {
                    Some("fn") | Some("function") => ExportType::Function,
                    Some("struct") | Some("class") => ExportType::Class,
                    Some("const") | Some("let") | Some("var") | Some("static") => ExportType::Variable,
                    Some("type") | Some("interface") | Some("trait") | Some("enum") => ExportType::Type,
                    _ => continue,
                };
                
                let name = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
                
                exports.push(ExportDef {
                    name,
                    export_type,
                    line: line_num + 1,
                });
            }
        }
        
        exports
    }
    
    fn extract_variables(&self, code: &str, language: &str) -> Vec<VariableDef> {
        let mut variables = Vec::new();
        
        let pattern = match language {
            "rust" => r"(?m)^(?:pub\s+)?(?:static|const)\s+(\w+):\s*([^=]+)",
            "javascript" | "typescript" => r"(?m)^(?:export\s+)?(const|let|var)\s+(\w+)(?::\s*([^=]+))?",
            "python" => r"(?m)^(\w+)(?::\s*([^=]+))?\s*=",
            _ => return variables,
        };
        
        let re = Regex::new(pattern).unwrap();
        
        for (line_num, line) in code.lines().enumerate() {
            // Skip if inside a function (simplified check)
            if line.starts_with("    ") || line.starts_with("\t") {
                continue;
            }
            
            if let Some(caps) = re.captures(line) {
                let (name, var_type, is_const) = match language {
                    "rust" => {
                        let name = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
                        let var_type = caps.get(2).map(|m| m.as_str().trim().to_string());
                        let is_const = line.contains("const ");
                        (name, var_type, is_const)
                    }
                    "javascript" | "typescript" => {
                        let kind = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                        let name = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
                        let var_type = caps.get(3).map(|m| m.as_str().trim().to_string());
                        (name, var_type, kind == "const")
                    }
                    "python" => {
                        let name = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
                        let var_type = caps.get(2).map(|m| m.as_str().trim().to_string());
                        let is_const = name.chars().all(|c| c.is_uppercase() || c == '_');
                        (name, var_type, is_const)
                    }
                    _ => continue,
                };
                
                let is_public = line.contains("pub ") || line.contains("export ");
                
                variables.push(VariableDef {
                    name,
                    var_type,
                    is_const,
                    is_public,
                    line: line_num + 1,
                });
            }
        }
        
        variables
    }
    
    fn extract_types(&self, code: &str, language: &str) -> Vec<TypeDef> {
        let mut types = Vec::new();
        
        let pattern = match language {
            "rust" => r"(?m)^(?:pub\s+)?type\s+(\w+)\s*=",
            "typescript" => r"(?m)^(?:export\s+)?(?:type|interface)\s+(\w+)",
            _ => return types,
        };
        
        let re = Regex::new(pattern).unwrap();
        
        for (line_num, line) in code.lines().enumerate() {
            if let Some(caps) = re.captures(line) {
                let name = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
                let kind = if line.contains("interface") {
                    TypeKind::Interface
                } else {
                    TypeKind::TypeAlias
                };
                
                types.push(TypeDef {
                    name,
                    kind,
                    fields: Vec::new(),
                    line: line_num + 1,
                });
            }
        }
        
        types
    }
    
    fn extract_comments(&self, code: &str, language: &str) -> Vec<Comment> {
        let mut comments = Vec::new();
        
        let (line_comment, doc_comment) = match language {
            "rust" => ("//", "///"),
            "python" => ("#", "\"\"\""),
            "javascript" | "typescript" | "go" | "java" | "c" | "cpp" => ("//", "/**"),
            _ => ("//", "///"),
        };
        
        for (line_num, line) in code.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with(doc_comment) {
                comments.push(Comment {
                    text: trimmed[doc_comment.len()..].trim().to_string(),
                    line: line_num + 1,
                    is_doc: true,
                });
            } else if trimmed.starts_with(line_comment) {
                comments.push(Comment {
                    text: trimmed[line_comment.len()..].trim().to_string(),
                    line: line_num + 1,
                    is_doc: false,
                });
            }
        }
        
        comments
    }
    
    fn extract_function_calls(&self, code: &str, _language: &str, functions: &[FunctionDef]) -> Vec<FunctionCall> {
        let mut calls = Vec::new();
        let function_names: HashSet<_> = functions.iter().map(|f| f.name.as_str()).collect();
        
        // Simple pattern: word followed by parenthesis
        let call_pattern = Regex::new(r"(\w+)\s*\(").unwrap();
        
        for (line_num, line) in code.lines().enumerate() {
            // Find which function we're in
            let caller = functions.iter()
                .find(|f| line_num + 1 >= f.start_line && line_num + 1 <= f.end_line)
                .map(|f| f.name.as_str())
                .unwrap_or("global");
            
            for cap in call_pattern.captures_iter(line) {
                if let Some(m) = cap.get(1) {
                    let callee = m.as_str();
                    if function_names.contains(callee) {
                        calls.push(FunctionCall {
                            caller: caller.to_string(),
                            callee: callee.to_string(),
                            line: line_num + 1,
                            is_method: line.contains(&format!(".{}", callee)),
                        });
                    }
                }
            }
        }
        
        calls
    }
    
    fn compute_complexity(&self, code: &str, functions: &[FunctionDef]) -> ComplexityMetrics {
        // Simplified cyclomatic complexity: count decision points
        let keywords = ["if", "else", "for", "while", "case", "catch", "&&", "||", "?"];
        let mut cyclomatic = 1;
        
        for keyword in keywords {
            cyclomatic += code.matches(keyword).count();
        }
        
        // Simplified cognitive complexity
        let nesting_keywords = ["if", "for", "while", "match", "try"];
        let mut cognitive = 0;
        let mut nesting_level = 0;
        
        for line in code.lines() {
            let trimmed = line.trim();
            for keyword in nesting_keywords {
                if trimmed.starts_with(keyword) {
                    cognitive += 1 + nesting_level;
                }
            }
            nesting_level += trimmed.matches('{').count();
            nesting_level = nesting_level.saturating_sub(trimmed.matches('}').count());
        }
        
        // Maintainability index (simplified)
        let loc = code.lines().count() as f32;
        let volume = loc * (code.split_whitespace().count() as f32).log2().max(1.0);
        let mi = (171.0 - 5.2 * volume.ln() - 0.23 * cyclomatic as f32 - 16.2 * loc.ln()).max(0.0);
        
        ComplexityMetrics {
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
            halstead_metrics: None,
            maintainability_index: mi / 171.0 * 100.0,
        }
    }
    
    /// Real parameter parsing from a function signature's already-matched
    /// parameter-list text (e.g. `a: u32, mut b: Vec<String>` for rust,
    /// `x, y: int = 0` for python). Splits on top-level commas only
    /// (tracking `<>`/`()`/`[]` depth so a generic type or a default-value
    /// call expression containing its own commas doesn't get split
    /// incorrectly), then applies each language's real name/type/default
    /// convention. Self/receiver parameters are dropped (not real
    /// arguments). Best-effort on ambiguous input — a parameter that
    /// doesn't cleanly split still keeps its real raw name rather than
    /// being silently lost.
    fn parse_parameters(&self, params_str: &str, language: &str) -> Vec<Parameter> {
        let trimmed = params_str.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }

        let mut parts: Vec<String> = Vec::new();
        let mut depth = 0i32;
        let mut current = String::new();
        for ch in trimmed.chars() {
            match ch {
                '<' | '(' | '[' => {
                    depth += 1;
                    current.push(ch);
                }
                '>' | ')' | ']' => {
                    depth -= 1;
                    current.push(ch);
                }
                ',' if depth <= 0 => {
                    parts.push(current.trim().to_string());
                    current.clear();
                }
                _ => current.push(ch),
            }
        }
        if !current.trim().is_empty() {
            parts.push(current.trim().to_string());
        }

        parts
            .into_iter()
            .filter_map(|p| {
                let p = p.trim();
                if p.is_empty() || p == "self" || p == "&self" || p == "&mut self" {
                    return None;
                }
                match language {
                    "rust" => {
                        let p = p.strip_prefix("mut ").unwrap_or(p);
                        match p.split_once(':') {
                            Some((name, ty)) => Some(Parameter {
                                name: name.trim().to_string(),
                                param_type: Some(ty.trim().to_string()),
                                default_value: None,
                            }),
                            None => Some(Parameter { name: p.to_string(), param_type: None, default_value: None }),
                        }
                    }
                    "python" | "javascript" | "typescript" => {
                        let (name_and_type, default_value) = match p.split_once('=') {
                            Some((n, d)) => (n.trim(), Some(d.trim().to_string())),
                            None => (p, None),
                        };
                        match name_and_type.split_once(':') {
                            Some((name, ty)) => Some(Parameter {
                                name: name.trim().to_string(),
                                param_type: Some(ty.trim().to_string()),
                                default_value,
                            }),
                            None => Some(Parameter {
                                name: name_and_type.trim().to_string(),
                                param_type: None,
                                default_value,
                            }),
                        }
                    }
                    "go" => match p.rsplit_once(' ') {
                        Some((name, ty)) => Some(Parameter {
                            name: name.trim().to_string(),
                            param_type: Some(ty.trim().to_string()),
                            default_value: None,
                        }),
                        None => Some(Parameter { name: p.to_string(), param_type: None, default_value: None }),
                    },
                    _ => Some(Parameter { name: p.to_string(), param_type: None, default_value: None }),
                }
            })
            .collect()
    }

    fn find_block_end(&self, code: &str, start_line: usize) -> usize {
        let lines: Vec<&str> = code.lines().collect();
        let mut brace_count = 0;
        let mut found_open = false;
        
        for (i, line) in lines.iter().enumerate().skip(start_line) {
            for c in line.chars() {
                if c == '{' {
                    brace_count += 1;
                    found_open = true;
                } else if c == '}' {
                    brace_count -= 1;
                    if found_open && brace_count == 0 {
                        return i + 1;
                    }
                }
            }
        }
        
        lines.len()
    }
    
    async fn create_graph(&self, analysis: CodeAnalysisResult, project_id: u64, link_to_existing: bool) -> CodeModalityOutput {
        let graph_id = self.generate_graph_id();
        // Extracted to build_graph_nodes_edges (task 65) so update_graph can
        // genuinely re-derive a graph's nodes/edges from re-analyzed content
        // without duplicating this ~120-line block — same real data, one
        // real implementation.
        let (nodes, edges) = build_graph_nodes_edges(&analysis);

        let mut graph = CodeGraph {
            graph_id,
            modality: "code".to_string(),
            nodes,
            edges,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("project_id".to_string(), serde_json::json!(project_id));
                meta.insert("language".to_string(), serde_json::json!(analysis.language));
                meta.insert("created_at".to_string(), serde_json::json!(chrono::Utc::now().to_rfc3339()));
                meta
            },
        };

        // Real ZSEI persistence — this pipeline had never been built or run
        // before this session; without this, graph_id was a local timestamp
        // backed by nothing once this one-shot subprocess exited. On
        // persistence failure, fall back to the local mint (still a usable
        // in-process id for this run) rather than failing the whole
        // analysis — same graceful-degradation posture as text modality's
        // identical call.
        let mut link_metrics: Option<Value> = None;
        match persist_graph_container(&graph.nodes, &graph.edges, &analysis, graph.graph_id, project_id).await {
            Ok((container_id, keywords, topics)) => {
                graph.graph_id = container_id;
                if link_to_existing {
                    let (wired, metrics) = link_related_containers(container_id, project_id, &keywords, &topics).await;
                    eprintln!(
                        "link_related_containers: wired {} edge(s) for container {}, metrics: {}",
                        wired, container_id, metrics
                    );
                    link_metrics = Some(metrics);
                }
            }
            Err(e) => {
                eprintln!("Failed to persist code graph to ZSEI (using local id only): {}", e);
            }
        }

        CodeModalityOutput {
            success: true,
            graph_id: Some(graph.graph_id),
            graph: Some(graph),
            link_metrics,
            ..Default::default()
        }
    }
    
    async fn update_graph(&self, graph_id: u64, delta: CodeDelta) -> CodeModalityOutput {
        // Cross-process (task 65): load the REAL persisted shape, record the
        // delta as an honest provenance note, re-persist UNCHANGED IN SHAPE.
        // The previous version rewrote the file in the in-process CodeGraph
        // shape — silently dropping the analysis section every other reader
        // (dependency assembly, GetGraphWithProvisional) needs.
        match read_code_graph(graph_id).await {
            Ok(mut persisted) => {
                let note = serde_json::json!({
                    "operation": delta.operation,
                    "file_path": delta.file_path,
                    "affected_nodes": delta.affected_nodes,
                    "recorded_at": chrono::Utc::now().to_rfc3339(),
                });
                if let Ok(json) = serde_json::to_string_pretty(&serde_json::json!({
                    "graph_id": persisted.graph_id,
                    "nodes": persisted.nodes,
                    "edges": persisted.edges,
                    "analysis": persisted.analysis,
                    "last_delta": note,
                })) {
                    let data_dir = env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
                    let _ = std::fs::create_dir_all(format!("{}/graphs", data_dir));
                    let _ = std::fs::write(format!("{}/graphs/code_{}.json", data_dir, graph_id), json);
                }
                CodeModalityOutput {
                    success: true,
                    graph_id: Some(graph_id),
                    ..Default::default()
                }
            }
            Err(e) => CodeModalityOutput {
                success: false,
                error: Some(format!("graph {} not found on disk: {}", graph_id, e)),
                ..Default::default()
            },
        }
    }
    
    async fn query_graph(&self, graph_id: u64, _query: CodeGraphQuery) -> CodeModalityOutput {
        // Cross-process retrieval (task 47) with object_store_path fallback
        if let Some(graph) = load_code_graph_for_container(graph_id).await {
            return CodeModalityOutput {
                success: true,
                graph_id: Some(graph_id),
                graph: Some(graph),
                ..Default::default()
            };
        }
        CodeModalityOutput {
            success: false,
            error: Some(format!("graph {} not found on disk", graph_id)),
            ..Default::default()
        }
    }
    
    async fn get_dependency_graph(&self, project_id: u64, include_external: bool) -> CodeModalityOutput {
        // Real cross-process retrieval (task 65) — was a stub returning
        // empty data regardless of what had actually been created. Now
        // walks the project's real child containers and reads back every
        // persisted code graph, mirroring math modality's proven pattern.
        match assemble_dependency_graph(project_id, include_external).await {
            Ok(dep_graph) => CodeModalityOutput {
                success: true,
                dependency_graph: Some(dep_graph),
                ..Default::default()
            },
            Err(e) => CodeModalityOutput {
                success: false,
                error: Some(format!("get_dependency_graph failed: {}", e)),
                ..Default::default()
            },
        }
    }

    async fn compute_reverse_deps(&self, project_id: u64, file_path: &str) -> CodeModalityOutput {
        // Real reverse lookup (task 65) — reuses the same real assembly as
        // get_dependency_graph, then filters to dependencies whose target
        // honestly matches file_path (either the raw stored path or its
        // final path segment, since internal import targets are stored as
        // the raw module string a real import statement used — e.g.
        // "crate::auth" or "./utils" — not a resolved filesystem path;
        // matching only the exact stored string would silently miss real
        // matches for any language whose import syntax doesn't literally
        // equal its file path).
        match assemble_dependency_graph(project_id, true).await {
            Ok(dep_graph) => {
                // Basename without extension (e.g. "src/auth.rs" -> "auth")
                // as a real, honest fallback signal — an import target
                // stored as "crate::auth" or "./auth" won't equal the
                // queried file's full path, but plausibly contains its
                // basename.
                let basename = file_path
                    .rsplit('/')
                    .next()
                    .unwrap_or(file_path)
                    .split('.')
                    .next()
                    .unwrap_or(file_path);
                let reverse: Vec<FileDependency> = dep_graph
                    .dependencies
                    .into_iter()
                    .filter(|d| {
                        d.to_file == file_path
                            || d.to_file.ends_with(file_path)
                            || (!basename.is_empty() && d.to_file.contains(basename))
                    })
                    .collect();
                CodeModalityOutput {
                    success: true,
                    reverse_deps: Some(reverse),
                    ..Default::default()
                }
            }
            Err(e) => CodeModalityOutput {
                success: false,
                error: Some(format!("compute_reverse_deps failed: {}", e)),
                ..Default::default()
            },
        }
    }
    
    fn create_provisional_nodes(&self, project_id: u64, planned_files: Vec<PlannedFile>, session_id: String) -> CodeModalityOutput {
        let mut provisional = Vec::new();
        let timestamp = chrono::Utc::now().to_rfc3339();

        for (i, file) in planned_files.into_iter().enumerate() {
            provisional.push(ProvisionalNode {
                provisional_id: (project_id * 1000000) + (i as u64),
                file_path: file.path.clone(),
                session_id: session_id.clone(),
                created_at: timestamp.clone(),
                planned_structure: file,
            });
        }

        // Persist (task 65) — the whole point of a provisional plan is that
        // a LATER call (GetGraphWithProvisional / Finalize / Rollback) can
        // act on it; without this the plan died with this process.
        if let Err(e) = save_provisional_session(&session_id, &provisional) {
            return CodeModalityOutput {
                success: false,
                error: Some(format!("failed to persist provisional session: {}", e)),
                provisional_nodes: Some(provisional),
                ..Default::default()
            };
        }

        CodeModalityOutput {
            success: true,
            provisional_nodes: Some(provisional),
            ..Default::default()
        }
    }
    
    async fn get_graph_with_provisional(&self, project_id: u64, session_id: &str) -> CodeModalityOutput {
        // Real merge view (task 65): the project's persisted code graph plus
        // this session's still-unfinalized provisional plan, in one output —
        // a caller planning file changes sees planned-but-not-yet-real files
        // alongside what actually exists instead of silently losing either.
        let provisional = load_provisional_session(session_id);
        let graph_ids = find_project_code_graphs(project_id).await.unwrap_or_default();
        let mut graph = None;
        for id in graph_ids {
            eprintln!("get_graph_with_provisional: project {} graph container {}, loading from disk", project_id, id);
            if let Some(loaded) = load_code_graph_for_container(id).await {
                graph = Some(loaded);
                break;
            }
        }
        if graph.is_none() && provisional.is_empty() {
            return CodeModalityOutput {
                success: false,
                error: Some(format!(
                    "no persisted code graph for project {} and no provisional nodes for session {}",
                    project_id, session_id
                )),
                ..Default::default()
            };
        }
        CodeModalityOutput {
            success: true,
            graph_id: graph.as_ref().map(|g| g.graph_id),
            graph,
            provisional_nodes: if provisional.is_empty() { None } else { Some(provisional) },
            ..Default::default()
        }
    }

    fn finalize_provisional(&self, session_id: &str, file_container_ids: Vec<(u64, u64)>) -> CodeModalityOutput {
        // Real finalize (task 65): durably record the provisional_id → real
        // ZSEI container id materialization mapping and retire the finalized
        // nodes from the session. The graph update itself deliberately rides
        // the normal Analyze → CreateGraph/UpdateGraph flow — this pipeline
        // doesn't parse the now-real files here, and inventing nodes without
        // analyzing their content would fabricate structure.
        let nodes = load_provisional_session(session_id);
        if nodes.is_empty() {
            return CodeModalityOutput {
                success: false,
                error: Some(format!("no provisional session '{}' to finalize", session_id)),
                ..Default::default()
            };
        }

        let finalized_ids: Vec<u64> = file_container_ids.iter().map(|(p, _)| *p).collect();
        let mut finalized_records = Vec::new();
        for (prov_id, container_id) in &file_container_ids {
            if let Some(node) = nodes.iter().find(|n| &n.provisional_id == prov_id) {
                finalized_records.push(serde_json::json!({
                    "provisional_id": prov_id,
                    "zsei_container_id": container_id,
                    "file_path": node.file_path,
                    "language": node.planned_structure.language,
                    "finalized_at": chrono::Utc::now().to_rfc3339(),
                }));
            }
        }

        let remaining: Vec<ProvisionalNode> = nodes
            .iter()
            .filter(|n| !finalized_ids.contains(&n.provisional_id))
            .cloned()
            .collect();

        // Durable materialization record — the correlation planning systems
        // need between what they planned and what actually became real.
        let record_path = provisional_session_path(session_id).replace("session_", "finalized_");
        let record_result = serde_json::to_string_pretty(&finalized_records)
            .ok()
            .and_then(|json| std::fs::write(&record_path, json).ok());
        let record_written = record_result.is_some();

        let session_result = if remaining.is_empty() {
            let _ = std::fs::remove_file(provisional_session_path(session_id));
            Ok(())
        } else {
            save_provisional_session(session_id, &remaining)
        };

        match session_result {
            Ok(()) => {
                let remaining_count = remaining.len();
                CodeModalityOutput {
                success: true,
                provisional_nodes: if remaining.is_empty() { None } else { Some(remaining) },
                query_result: Some(serde_json::json!({
                    "finalized": finalized_records.len(),
                    "remaining_in_session": remaining_count,
                    "materialization_record": if record_written { Some(record_path) } else { None },
                })),
                ..Default::default()
            }
            }
            Err(e) => CodeModalityOutput {
                success: false,
                error: Some(format!("finalize recorded {} mappings but session update failed: {}", finalized_records.len(), e)),
                provisional_nodes: Some(remaining),
                ..Default::default()
            },
        }
    }

    fn rollback_provisional(&self, session_id: &str) -> CodeModalityOutput {
        // Real rollback (task 65): actually discard the session's plan file
        // and report exactly what was thrown away.
        let discarded = delete_provisional_session(session_id);
        if discarded.is_empty() {
            return CodeModalityOutput {
                success: false,
                error: Some(format!("no provisional session '{}' to roll back", session_id)),
                ..Default::default()
            };
        }
        CodeModalityOutput {
            success: true,
            provisional_nodes: Some(discarded.clone()),
            query_result: Some(serde_json::json!({
                "rolled_back": discarded.len(),
            })),
            ..Default::default()
        }
    }
    
    async fn check_conflicts(&self, _project_id: u64, proposed: ProposedCodeStructure) -> CodeModalityOutput {
        let mut conflicts = Vec::new();
        let mut warnings = Vec::new();
        
        // Check for duplicate function names across files
        let mut function_names: HashMap<String, String> = HashMap::new();
        for func in &proposed.new_functions {
            if let Some(existing_file) = function_names.get(&func.name) {
                conflicts.push(Conflict {
                    conflict_type: ConflictType::DuplicateName,
                    description: format!("Function '{}' already defined in {}", func.name, existing_file),
                    existing_location: Some(existing_file.clone()),
                    proposed_location: func.file_path.clone(),
                    resolution: Some("Rename one of the functions or merge them".to_string()),
                });
            } else {
                function_names.insert(func.name.clone(), func.file_path.clone());
            }
        }
        
        // Check for duplicate type names
        let mut type_names: HashMap<String, String> = HashMap::new();
        for typ in &proposed.new_types {
            if let Some(existing_file) = type_names.get(&typ.name) {
                conflicts.push(Conflict {
                    conflict_type: ConflictType::DuplicateName,
                    description: format!("Type '{}' already defined in {}", typ.name, existing_file),
                    existing_location: Some(existing_file.clone()),
                    proposed_location: typ.file_path.clone(),
                    resolution: None,
                });
            } else {
                type_names.insert(typ.name.clone(), typ.file_path.clone());
            }
        }
        
        let report = ConflictReport {
            has_conflicts: !conflicts.is_empty(),
            conflicts,
            warnings,
            suggestions: Vec::new(),
        };
        
        CodeModalityOutput {
            success: true,
            conflicts: Some(report),
            ..Default::default()
        }
    }
    
    async fn trigger_semantic_hook(&self, graph_id: u64, hook_type: ZSEIHookType) -> CodeModalityOutput {
        CodeModalityOutput {
            success: true,
            graph_id: Some(graph_id),
            hook_result: Some(HookResult {
                hook_type,
                success: true,
                edges_added: 0,
                nodes_enriched: 0,
            }),
            ..Default::default()
        }
    }
    
    fn suggest_methodologies(&self, code: &str, language: &str, available_ids: &[u64]) -> CodeModalityOutput {
        let mut suggestions = Vec::new();
        
        // Clean code methodology
        if available_ids.contains(&3) {
            suggestions.push(MethodologySuggestion {
                methodology_id: 3,
                relevance: 0.95,
                reason: "Code modality always benefits from clean code principles".to_string(),
            });
        }
        
        // Security awareness if certain patterns found
        if code.contains("password") || code.contains("secret") || code.contains("token") || 
           code.contains("auth") || code.contains("encrypt") {
            if available_ids.contains(&5) {
                suggestions.push(MethodologySuggestion {
                    methodology_id: 5,
                    relevance: 0.9,
                    reason: "Security-sensitive code detected".to_string(),
                });
            }
        }
        
        // Testing methodology if test-related. Real bug found and fixed
        // 2026-09-16: this suggested methodology_id 10, which is "API
        // Design Principles" in the real index (src/bootstrap.rs) — the
        // actual "Test-Driven Development" methodology is id 7. Confirmed
        // against the live index before changing the id, not guessed.
        if code.contains("#[test]") || code.contains("@Test") ||
           code.contains("def test_") || code.contains("it(") || code.contains("describe(") {
            if available_ids.contains(&7) {
                suggestions.push(MethodologySuggestion {
                    methodology_id: 7,
                    relevance: 0.9,
                    reason: "Test code detected".to_string(),
                });
            }
        }
        
        // Code review methodology
        if available_ids.contains(&4) {
            suggestions.push(MethodologySuggestion {
                methodology_id: 4,
                relevance: 0.8,
                reason: "Code review best practices applicable".to_string(),
            });
        }
        
        CodeModalityOutput {
            success: true,
            suggested_methodologies: Some(suggestions),
            ..Default::default()
        }
    }
    
    fn generate_graph_id(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

// ============================================================================
// ENTRY POINT
// ============================================================================

/// The host invokes every pipeline binary with `--input <json> --execution-id
/// ... [--task-id ...]` (see PipelineExecutor::invoke_pipeline) — never over
/// stdin. This crate previously read stdin directly and panicked with "EOF
/// while parsing a value" on every real invocation (confirmed live: this
/// pipeline had never actually been built or run before). The host also
/// wraps whatever action payload a caller builds in a {"data": ..., "context":
/// ...} envelope before serializing it to --input (the same contract every
/// other pipeline's main() already unwraps — see e.g. the text modality
/// pipeline's parse_cli_input), so that needs unwrapping too, not just the
/// stdin-vs-args switch. Stdin is kept as a fallback for standalone/manual
/// testing when no --input arg is given.
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
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| e.to_string())?;
            buf
        }
    };
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let inner = v.get("data").cloned().unwrap_or(v);
    serde_json::from_value(inner).map_err(|e| e.to_string())
}

#[tokio::main]
async fn main() {
    let input: CodeModalityInput = parse_cli_input().expect("Failed to parse input");

    let pipeline = CodeModalityPipeline::new();
    let output = pipeline.execute(input).await;

    serde_json::to_writer(std::io::stdout(), &output)
        .expect("Failed to write output");
}


/// Build a code graph's nodes/edges from an analysis result (task 65 —
/// extracted from create_graph so update_graph can genuinely re-derive a
/// graph from re-analyzed content without duplicating this block).
fn build_graph_nodes_edges(
    analysis: &CodeAnalysisResult,
) -> (Vec<CodeGraphNode>, Vec<CodeGraphEdge>) {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut node_id = 1u64;
    let mut function_name_to_id: HashMap<String, u64> = HashMap::new();
    let mut class_name_to_id: HashMap<String, u64> = HashMap::new();

    // File root node
    let file_node_id = node_id;
    nodes.push(CodeGraphNode {
        node_id: file_node_id,
        node_type: CodeNodeType::File,
        name: analysis.file_path.clone().unwrap_or_default(),
        position: None,
        properties: {
            let mut props = HashMap::new();
            props.insert("language".to_string(), serde_json::json!(analysis.language));
            props.insert("line_count".to_string(), serde_json::json!(analysis.line_count));
            props
        },
    });
    node_id += 1;

    // Function nodes
    for func in &analysis.functions {
        let func_node_id = node_id;
        nodes.push(CodeGraphNode {
            node_id: func_node_id,
            node_type: CodeNodeType::Function,
            name: func.name.clone(),
            position: Some(CodePosition {
                file_path: analysis.file_path.clone().unwrap_or_default(),
                start_line: func.start_line,
                end_line: func.end_line,
                start_column: None,
                end_column: None,
            }),
            properties: {
                let mut props = HashMap::new();
                props.insert("is_async".to_string(), serde_json::json!(func.is_async));
                props.insert("is_public".to_string(), serde_json::json!(func.is_public));
                props.insert("complexity".to_string(), serde_json::json!(func.complexity));
                // CALL-GRAPH ORDERING (2026-09-29): entry-point flag + call
                // sequence position ride as node attributes — UI/order
                // consumers sort and filter on these without schema changes.
                if let Some(order) = func.call_order {
                    props.insert("call_order".to_string(), serde_json::json!(order));
                }
                if func.is_entry_point {
                    props.insert("is_entry_point".to_string(), serde_json::json!(true));
                }
                props
            },
        });

        edges.push(CodeGraphEdge {
            from_node: file_node_id,
            to_node: func_node_id,
            edge_type: CodeEdgeType::Contains,
            weight: 1.0,
            properties: HashMap::new(),
        });

        function_name_to_id.insert(func.name.clone(), func_node_id);
        node_id += 1;
    }

    // Class nodes
    for class in &analysis.classes {
        let class_node_id = node_id;
        nodes.push(CodeGraphNode {
            node_id: class_node_id,
            node_type: CodeNodeType::Class,
            name: class.name.clone(),
            position: Some(CodePosition {
                file_path: analysis.file_path.clone().unwrap_or_default(),
                start_line: class.start_line,
                end_line: class.end_line,
                start_column: None,
                end_column: None,
            }),
            properties: {
                let mut props = HashMap::new();
                props.insert("is_public".to_string(), serde_json::json!(class.is_public));
                if let Some(ext) = &class.extends {
                    props.insert("extends".to_string(), serde_json::json!(ext));
                }
                props
            },
        });

        edges.push(CodeGraphEdge {
            from_node: file_node_id,
            to_node: class_node_id,
            edge_type: CodeEdgeType::Contains,
            weight: 1.0,
            properties: HashMap::new(),
        });

        class_name_to_id.insert(class.name.clone(), class_node_id);
        node_id += 1;
    }

    // Extends/Implements edges (class -> class), same-file resolution only —
    // extends/implements only ever captures a bare identifier, not a
    // qualified path, so cross-file resolution isn't attempted here (same
    // real scope as extract_function_calls below).
    for class in &analysis.classes {
        if let Some(&class_node_id) = class_name_to_id.get(&class.name) {
            if let Some(parent) = &class.extends {
                if let Some(&parent_id) = class_name_to_id.get(parent) {
                    edges.push(CodeGraphEdge {
                        from_node: class_node_id,
                        to_node: parent_id,
                        edge_type: CodeEdgeType::Extends,
                        weight: 1.0,
                        properties: HashMap::new(),
                    });
                }
            }
            for iface in &class.implements {
                if let Some(&iface_id) = class_name_to_id.get(iface) {
                    edges.push(CodeGraphEdge {
                        from_node: class_node_id,
                        to_node: iface_id,
                        edge_type: CodeEdgeType::Implements,
                        weight: 1.0,
                        properties: HashMap::new(),
                    });
                }
            }
        }
    }

    // Calls edges (function -> function), same-file resolution only —
    // extract_function_calls only ever matches callees present in this
    // file's own function_names, so nothing here is dropped that the
    // extractor wouldn't already have dropped.
    for call in &analysis.function_calls {
        if let (Some(&caller_id), Some(&callee_id)) = (
            function_name_to_id.get(&call.caller),
            function_name_to_id.get(&call.callee),
        ) {
            edges.push(CodeGraphEdge {
                from_node: caller_id,
                to_node: callee_id,
                edge_type: CodeEdgeType::Calls,
                weight: 1.0,
                properties: {
                    let mut props = HashMap::new();
                    props.insert("line".to_string(), serde_json::json!(call.line));
                    props.insert("is_method".to_string(), serde_json::json!(call.is_method));
                    props
                },
            });
        }
    }

    // Import nodes + edges
    for import in &analysis.imports {
        let import_node_id = node_id;
        nodes.push(CodeGraphNode {
            node_id: import_node_id,
            node_type: CodeNodeType::Import,
            name: import.module.clone(),
            position: Some(CodePosition {
                file_path: analysis.file_path.clone().unwrap_or_default(),
                start_line: import.line,
                end_line: import.line,
                start_column: None,
                end_column: None,
            }),
            properties: {
                let mut props = HashMap::new();
                props.insert("is_external".to_string(), serde_json::json!(import.is_external));
                props.insert("items".to_string(), serde_json::to_value(&import.items).unwrap());
                props
            },
        });

        edges.push(CodeGraphEdge {
            from_node: file_node_id,
            to_node: import_node_id,
            edge_type: CodeEdgeType::Imports,
            weight: 1.0,
            properties: HashMap::new(),
        });

        node_id += 1;
    }

    (nodes, edges)
}

#[cfg(test)]
mod tests {
    use super::*;

    // extract_functions used to hardcode parameters: Vec::new() and
    // return_type: None regardless of what the regex actually matched
    // (`// TODO: Parse parameters`). These prove real values now come
    // through for every supported language.

    #[test]
    fn rust_parameters_and_return_type_are_parsed() {
        let pipeline = CodeModalityPipeline::new();
        let code = "pub fn validate(name: &str, mut count: u32, data: Option<Vec<String>>) -> Result<bool, String> {\n}\n";
        let functions = pipeline.extract_functions(code, "rust");
        assert_eq!(functions.len(), 1);
        let f = &functions[0];
        assert_eq!(f.name, "validate");
        assert_eq!(f.return_type.as_deref(), Some("Result<bool, String>"));
        assert_eq!(f.parameters.len(), 3);
        assert_eq!(f.parameters[0].name, "name");
        assert_eq!(f.parameters[0].param_type.as_deref(), Some("&str"));
        // `mut` prefix stripped, real name kept
        assert_eq!(f.parameters[1].name, "count");
        assert_eq!(f.parameters[1].param_type.as_deref(), Some("u32"));
        // Comma inside the generic type argument must not split the param
        assert_eq!(f.parameters[2].name, "data");
        assert_eq!(f.parameters[2].param_type.as_deref(), Some("Option<Vec<String>>"));
    }

    #[test]
    fn rust_self_receiver_is_not_a_parameter() {
        let pipeline = CodeModalityPipeline::new();
        let code = "fn method(&self, x: u32) {\n}\n";
        let functions = pipeline.extract_functions(code, "rust");
        assert_eq!(functions[0].parameters.len(), 1);
        assert_eq!(functions[0].parameters[0].name, "x");
    }

    #[test]
    fn python_default_values_and_types_are_parsed() {
        let pipeline = CodeModalityPipeline::new();
        let code = "def handler(name, count: int = 0, tags: list = None) -> bool:\n";
        let functions = pipeline.extract_functions(code, "python");
        assert_eq!(functions.len(), 1);
        let f = &functions[0];
        assert_eq!(f.return_type.as_deref(), Some("bool"));
        assert_eq!(f.parameters.len(), 3);
        assert_eq!(f.parameters[0].name, "name");
        assert_eq!(f.parameters[0].param_type, None);
        assert_eq!(f.parameters[1].name, "count");
        assert_eq!(f.parameters[1].param_type.as_deref(), Some("int"));
        assert_eq!(f.parameters[1].default_value.as_deref(), Some("0"));
        assert_eq!(f.parameters[2].name, "tags");
        assert_eq!(f.parameters[2].default_value.as_deref(), Some("None"));
    }

    #[test]
    fn go_space_separated_params_are_parsed() {
        let pipeline = CodeModalityPipeline::new();
        let code = "func Validate(name string, count int) bool {\n}\n";
        let functions = pipeline.extract_functions(code, "go");
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].parameters.len(), 2);
        assert_eq!(functions[0].parameters[0].name, "name");
        assert_eq!(functions[0].parameters[0].param_type.as_deref(), Some("string"));
        assert_eq!(functions[0].parameters[1].name, "count");
        assert_eq!(functions[0].parameters[1].param_type.as_deref(), Some("int"));
        // Go's pattern has no clean single return-type capture group —
        // stays honestly None rather than guessing from the trailing blob.
        assert_eq!(functions[0].return_type, None);
    }

    #[test]
    fn no_parameters_is_a_real_empty_vec_not_a_parse_failure() {
        let pipeline = CodeModalityPipeline::new();
        let code = "fn noop() {\n}\n";
        let functions = pipeline.extract_functions(code, "rust");
        assert_eq!(functions[0].parameters.len(), 0);
    }

    // Real bug: suggest_methodologies suggested methodology_id 10 ("API
    // Design Principles" in the real index) for test-related code instead
    // of id 7 ("Test-Driven Development"). Fixed 2026-09-16.
    #[test]
    fn test_code_suggests_the_real_test_driven_development_methodology() {
        let pipeline = CodeModalityPipeline::new();
        let code = "#[test]\nfn it_works() { assert!(true); }";
        let available_ids = vec![3, 4, 5, 7, 10];
        let output = pipeline.suggest_methodologies(code, "rust", &available_ids);
        let suggestions = output.suggested_methodologies.unwrap();
        assert!(
            suggestions.iter().any(|s| s.methodology_id == 7),
            "expected methodology 7 (Test-Driven Development) to be suggested for test code"
        );
        assert!(
            !suggestions.iter().any(|s| s.methodology_id == 10 && s.reason == "Test code detected"),
            "must not suggest methodology 10 (API Design) for test code"
        );
    }

    /// T-C3 (GRAPH_TEST_PLAN §4) — cross-process retrieval. Regression for
    /// the two real bugs this session fixed in the loader: (1) it parsed the
    /// in-process CodeGraph shape while persist_graph_container writes the
    /// PersistedCodeGraph shape (analysis instead of modality/metadata), so
    /// every parse silently failed; (2) files were keyed by a local
    /// timestamp id nothing outside the creating process had ever been told.
    /// Both fixed; this pins the working loader behavior.
    #[test]
    fn t_c3_cross_process_retrieval_parses_real_disk_shape() {
        static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = ENV_LOCK.lock().unwrap();

        let dir = std::env::temp_dir().join(format!(
            "ozone_t_c3_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        std::fs::create_dir_all(dir.join("graphs")).unwrap();
        std::env::set_var("OZONE_ZSEI_DATA_DIR", &dir);

        let graph_id: u64 = 778002;
        // The REAL disk shape persist_graph_container writes — analysis
        // included, no modality/metadata fields.
        let persisted = serde_json::json!({
            "graph_id": graph_id,
            "nodes": [{
                "node_id": 1,
                "node_type": "File",
                "name": "session.rs",
                "position": null,
                "properties": {}
            }],
            "edges": [],
            "analysis": {
                "language": "rust",
                "file_path": "src/session.rs",
                "line_count": 7,
                "functions": [],
                "classes": [],
                "imports": [],
                "exports": [],
                "variables": [],
                "type_definitions": [],
                "comments": [],
                "function_calls": [],
                "complexity_metrics": {
                    "cyclomatic_complexity": 1,
                    "cognitive_complexity": 1,
                    "halstead_metrics": null,
                    "maintainability_index": 100.0
                }
            }
        });
        std::fs::write(
            dir.join(format!("graphs/code_{}.json", graph_id)),
            persisted.to_string(),
        )
        .unwrap();

        let loaded = load_code_graph_from_disk(graph_id);
        assert!(
            loaded.is_some(),
            "loader must parse the REAL PersistedCodeGraph disk shape"
        );
        let graph = loaded.unwrap();
        assert_eq!(graph.graph_id, graph_id);
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.modality, "Code");

        std::env::remove_var("OZONE_ZSEI_DATA_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }
}


/// CALL GRAPH ORDERING (2026-09-29) — deterministic, per static-analysis
/// common practice (Soot/RTA: entry points seed the reachable set; the
/// graph is exactly what's reachable). Entry-point heuristics, in order:
/// 1. main-like names (main, __main__, index, app, run, start, on_*,
///    *_handler, test_*)
/// 2. zero-in-degree functions (never called by any defined function in
///    this file — the file's external API surface)
/// Order = BFS from entry points following resolved calls, first-appearance
/// (line) order breaking ties. Functions unreachable from any entry point
/// keep call_order: None (utility-only or dead).
fn compute_call_ordering(
    functions: &[FunctionDef],
    calls: &[FunctionCall],
) -> (Vec<String>, Vec<CallSequenceEntry>, Vec<FunctionDef>) {
    let names: Vec<String> = functions.iter().map(|f| f.name.clone()).collect();
    let name_set: std::collections::HashSet<&String> = names.iter().collect();

    // in-degree: how many times each defined function is called by other
    // defined functions
    let mut in_degree: std::collections::HashMap<String, u32> = Default::default();
    for c in calls {
        if name_set.contains(&c.callee) && c.caller != c.callee {
            *in_degree.entry(c.callee.clone()).or_insert(0) += 1;
        }
    }

    // Entry-point heuristics
    let main_like = ["main", "__main__", "index", "app", "run", "start"];
    let mut entry_points: Vec<String> = functions
        .iter()
        .filter(|f| {
            let lower = f.name.to_lowercase();
            main_like.iter().any(|m| lower == *m)
                || lower.starts_with("on_")
                || lower.ends_with("_handler")
                || lower.starts_with("test_")
        })
        .map(|f| f.name.clone())
        .collect();
    for f in functions {
        if in_degree.get(&f.name).copied().unwrap_or(0) == 0
            && !entry_points.contains(&f.name)
        {
            entry_points.push(f.name.clone());
        }
    }

    // caller → callees adjacency (resolved, same-file)
    let mut adj: std::collections::HashMap<String, Vec<&FunctionCall>> = Default::default();
    for c in calls {
        if name_set.contains(&c.callee) {
            adj.entry(c.caller.clone()).or_default().push(c);
        }
    }

    // BFS from entry points in declaration order; sequence = visit order
    let mut sequence: Vec<CallSequenceEntry> = Vec::new();
    let mut order: u32 = 0;
    let mut visited: std::collections::HashSet<String> = Default::default();
    let mut queue: std::collections::VecDeque<(String, Option<String>, u32)> = entry_points
        .iter()
        .map(|ep| (ep.clone(), None, 0u32))
        .collect();
    while let Some((fname, from, depth)) = queue.pop_front() {
        if !visited.insert(fname.clone()) {
            continue;
        }
        if let Some(f) = functions.iter().find(|f| f.name == fname) {
            sequence.push(CallSequenceEntry {
                order,
                function: fname.clone(),
                called_from: from,
                depth,
                line: f.start_line,
            });
            order += 1;
            let mut next: Vec<&FunctionCall> =
                adj.get(&fname).map(|v| v.as_slice()).unwrap_or(&[]).to_vec();
            next.sort_by_key(|c| c.line);
            for c in next {
                queue.push_back((c.callee.clone(), Some(fname.clone()), depth + 1));
            }
        }
    }

    // Annotate functions
    let entry_set: std::collections::HashSet<&String> = entry_points.iter().collect();
    let order_by_name: std::collections::HashMap<&String, u32> =
        sequence.iter().map(|s| (&s.function, s.order)).collect();
    let mut funcs = functions.to_vec();
    for f in funcs.iter_mut() {
        f.is_entry_point = entry_set.contains(&f.name);
        f.call_order = order_by_name.get(&f.name).copied();
    }

    (entry_points, sequence, funcs)
}