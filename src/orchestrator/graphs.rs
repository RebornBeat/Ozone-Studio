//! //! Modality-graph creation and maintenance — root modality aggregation,
//! file-graph classification, initial graph creation (structural → semantic →
//! cross-modal until stable), and the modality-name pipeline mapping.

use super::*;

impl PromptOrchestrator {

    /// Aggregate modality detections from all processed chunk graphs into the root modality list.
    /// Verifies each modality has a registered pipeline handler.
    pub(crate) async fn aggregate_root_modalities(&self, state: &mut OrchestrationState) {
        let mut modality_evidence: HashMap<String, Vec<ModalityEvidence>> = HashMap::new();

        for chunk in &state.processed_chunks {
            for detection in &chunk.detected_modalities {
                // true_text / unknown are never emitted anymore — the text
                // pipeline guarantees it (prose is covered by sentence
                // extraction; only non-prose spans are reported).
                modality_evidence
                    .entry(detection.modality.clone())
                    .or_default()
                    .push(ModalityEvidence {
                        chunk_index: chunk.index,
                        span_start: detection.span_start,
                        span_end: detection.span_end,
                        intent_reference: detection.intent_reference.clone(),
                    });
            }
        }

        // Also include modalities from file graphs
        for classified in &state.classified_file_graphs {
            modality_evidence
                .entry(classified.modality.clone())
                .or_default()
                .push(ModalityEvidence {
                    chunk_index: u32::MAX, // sentinel: from file, not chunk
                    span_start: 0,
                    span_end: 0,
                    intent_reference: format!("file:{}", classified.file_path),
                });
        }

        // Always include text
        if !modality_evidence.contains_key("text") {
            modality_evidence.insert("text".to_string(), vec![]);
        }

        let mut verified = Vec::new();
        let mut unhandled = Vec::new();

        for (modality, evidence) in modality_evidence {
            let pipeline_id = self.modality_name_to_pipeline_id(&modality);
            if pipeline_id > 0 {
                verified.push(VerifiedModality {
                    modality,
                    pipeline_id,
                    evidence,
                });
            } else {
                unhandled.push(UnhandledModality { modality, evidence });
            }
        }

        state.root_modality_list = RootModalityList {
            verified_modalities: verified,
            unhandled_modalities: unhandled,
            total_chunk_count: state.processed_chunks.len() as u32,
        };
    }

    /// Phase 2: Create all initial modality graphs before AMT building.
    /// Three passes: structural (parallel) → semantic enrichment (text-first) →
    /// cross-modal reference (iterative 5x stable).
    pub(crate) async fn create_initial_modality_graphs(
        &self,
        state: &mut OrchestrationState,
    ) -> Result<(), String> {
        // PASS 1: Structural creation
        let verified = state.root_modality_list.verified_modalities.clone();

        for vm in &verified {
            let modality = vm.modality.clone();
            let pipeline_id = vm.pipeline_id;
            let project_id = state.request.project_id.unwrap_or(0);

            let modality_text: String = state
                .processed_chunks
                .iter()
                .flat_map(|chunk| {
                    chunk
                        .detected_modalities
                        .iter()
                        .filter(|d| d.modality == modality)
                        .map(|d| {
                            let end = d.span_end.min(chunk.cleaned_text.len());
                            chunk.cleaned_text[d.span_start.min(end)..end].to_string()
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
                .join("\n");

            let analyze_input = serde_json::json!({
                "action": {
                    "type": "Analyze",
                    "text": if modality == "text" {
                        state.cleaned_prompt.clone()
                    } else {
                        modality_text
                    },
                    "depth": "Standard",
                    "extract_entities": true,
                    "extract_topics": true,
                    "extract_structure": false
                }
            });

            let analysis = self
                .executor
                .execute(pipeline_id, analyze_input)
                .await
                .unwrap_or_default();

            let graph_input = serde_json::json!({
                "action": {
                    "type": "CreateGraph",
                    "analysis_result": analysis.get("analysis").cloned().unwrap_or_default(),
                    "project_id": project_id,
                    "link_to_existing": false
                }
            });

            let graph_result = self
                .executor
                .execute(pipeline_id, graph_input)
                .await
                .unwrap_or_default();

            let graph_id = graph_result
                .get("graph_id")
                .and_then(|g| g.as_u64())
                .unwrap_or(Self::generate_id_static());

            state.modality_graphs.insert(modality.clone(), graph_id);
            state.graph_states.insert(
                graph_id,
                SessionGraphState {
                    graph_id,
                    modality: modality.clone(),
                    pipeline_id,
                    state: ModalityGraphState::Created,
                    cross_modal_edge_count: 0,
                    consecutive_stable_passes: 0,
                },
            );
        }

        // PASS 2: Semantic enrichment (text-first)
        if let Some(&text_graph_id) = state.modality_graphs.get("text") {
            let text_pipeline_id = self.modality_name_to_pipeline_id("text");
            let hook_input = serde_json::json!({
                "action": {
                    "type": "TriggerSemanticHook",
                    "graph_id": text_graph_id,
                    "hook_type": "OnInferRelationships"
                }
            });
            let _ = self.executor.execute(text_pipeline_id, hook_input).await;
            if let Some(gs) = state.graph_states.get_mut(&text_graph_id) {
                gs.state = ModalityGraphState::SemanticEnriched;
            }
        }

        let other_graphs: Vec<(String, u64)> = state
            .modality_graphs
            .iter()
            .filter(|(k, _)| *k != "text")
            .map(|(k, v)| (k.clone(), *v))
            .collect();

        for (modality, graph_id) in &other_graphs {
            let pipeline_id = self.modality_name_to_pipeline_id(modality);
            let hook_input = serde_json::json!({
                "action": {
                    "type": "TriggerSemanticHook",
                    "graph_id": graph_id,
                    "hook_type": "OnInferRelationships"
                }
            });
            let _ = self.executor.execute(pipeline_id, hook_input).await;
            if let Some(gs) = state.graph_states.get_mut(graph_id) {
                gs.state = ModalityGraphState::SemanticEnriched;
            }
        }

        // PASS 3: Cross-modal references
        self.build_cross_modal_references_until_stable(state)
            .await?;

        state.initial_graphs_created = true;

        // Register in task store
        if let Some(task_id) = state.task_id {
            let register_input = serde_json::json!({
                "action": "RegisterGraphs",
                "task_id": task_id,
                "modality_graph_ids": state.modality_graphs,
            });
            let _ = self.executor.execute(5, register_input).await;
        }

        Ok(())
    }

    /// Iterative cross-modal reference building with 5x consecutive stable termination.
    async fn build_cross_modal_references_until_stable(
        &self,
        state: &mut OrchestrationState,
    ) -> Result<(), String> {
        let mut stable_count = 0u32;
        let mut pass = 0u32;

        loop {
            pass += 1;
            let new_edges = self.run_cross_modal_reference_pass(state, pass).await;

            if new_edges == 0 {
                stable_count += 1;
                if stable_count >= 5 {
                    break;
                }
            } else {
                stable_count = 0;
            }

            if pass > 30 {
                break;
            } // safety ceiling
        }

        // Mark all graphs CrossLinked → Stable
        for gs in state.graph_states.values_mut() {
            gs.state = ModalityGraphState::Stable;
            gs.consecutive_stable_passes = 5;
        }

        Ok(())
    }

    /// Single cross-modal reference pass. Returns count of new edges discovered.
    /// Pass 1-2: text ↔ others. Pass 3+: inter-modality.
    async fn run_cross_modal_reference_pass(
        &self,
        state: &mut OrchestrationState,
        pass_number: u32,
    ) -> usize {
        let mut new_edges = 0usize;

        let text_graph_id = state.modality_graphs.get("text").copied();
        let all_graphs: Vec<(String, u64)> = state
            .modality_graphs
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();

        // Phase A: Text → all other modalities
        if let Some(text_gid) = text_graph_id {
            for (target_modality, target_gid) in all_graphs.iter().filter(|(k, _)| k != "text") {
                let text_pipeline = self.modality_name_to_pipeline_id("text");
                let link_input = serde_json::json!({
                    "action": {
                        "type": "LinkToModality",
                        "source_graph_id": text_gid,
                        "target_graph_id": target_gid,
                        "target_modality": target_modality,
                        "relationship": "ReferencesModality"
                    }
                });
                if let Ok(result) = self.executor.execute(text_pipeline, link_input).await {
                    if result.get("link_result").is_some() {
                        new_edges += 1;
                        if let Some(gs) = state.graph_states.get_mut(target_gid) {
                            gs.cross_modal_edge_count += 1;
                        }
                    }
                }
            }
        }

        // Phase B: All → text back-references (pass 2+)
        if pass_number >= 2 {
            if let Some(text_gid) = text_graph_id {
                for (source_modality, source_gid) in all_graphs.iter().filter(|(k, _)| k != "text")
                {
                    let source_pipeline = self.modality_name_to_pipeline_id(source_modality);
                    let link_input = serde_json::json!({
                        "action": {
                            "type": "LinkToModality",
                            "source_graph_id": source_gid,
                            "target_graph_id": text_gid,
                            "target_modality": "text",
                            "relationship": "ReferencedBy"
                        }
                    });
                    if let Ok(result) = self.executor.execute(source_pipeline, link_input).await {
                        if result.get("link_result").is_some() {
                            new_edges += 1;
                        }
                    }
                }
            }
        }

        // Phase C: Inter-modality (pass 3+)
        if pass_number >= 3 {
            for i in 0..all_graphs.len() {
                for j in (i + 1)..all_graphs.len() {
                    let (a_mod, a_gid) = &all_graphs[i];
                    let (b_mod, b_gid) = &all_graphs[j];
                    if a_mod == "text" || b_mod == "text" {
                        continue;
                    }

                    let a_pipeline = self.modality_name_to_pipeline_id(a_mod);
                    let link_input = serde_json::json!({
                        "action": {
                            "type": "LinkToModality",
                            "source_graph_id": a_gid,
                            "target_graph_id": b_gid,
                            "target_modality": b_mod,
                            "relationship": "RelatesTo"
                        }
                    });
                    if let Ok(result) = self.executor.execute(a_pipeline, link_input).await {
                        if result.get("link_result").is_some() {
                            new_edges += 1;
                        }
                    }
                }
            }
        }

        new_edges
    }

    /// Classify file graphs as primary / supplementary / raw_data.
    ///
    /// CRITICAL: This is called AFTER file graphs AND the text graph already exist.
    /// It operates on GRAPH METADATA only — never on raw file content.
    /// This eliminates the chunk-size assumption entirely.
    pub(crate) async fn classify_file_graphs_post_creation(
        &self,
        state: &mut OrchestrationState,
    ) -> Result<(), String> {
        if state.file_graphs.is_empty() {
            return Ok(());
        }

        // 1. Extract text graph intent summary from metadata (keywords + topics, never raw text)
        let text_intent_summary = format!(
            "Prompt keywords: [{}]. Topics: [{}]. AMT intent: {}",
            state
                .keywords
                .iter()
                .take(15)
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
            state
                .topics
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
            state
                .amt
                .as_ref()
                .map(|a| a.content.as_str())
                .unwrap_or("not yet built"),
        );

        // 2. Collect file graph summaries — metadata only, NOT raw file content
        let file_summaries: Vec<serde_json::Value> = state.file_graphs.iter().map(|(path, &graph_id)| {
            let modality = self.detect_file_modality(path);
            serde_json::json!({
                "file_path": path,
                "graph_id": graph_id,
                "modality": modality,
                "graph_summary": format!("File graph {} for {} file: {}", graph_id, modality, path)
            })
        }).collect();

        // 3. LLM: compare file graph metadata against prompt intent
        let prompt = format!(
            r#"Given the prompt's intent summary and a set of file graph metadata,
    classify each file graph's role relative to the prompt.

    Prompt intent:
    {}

    File graphs (metadata only — no raw file content):
    {}

    For each file, classify as:
    - "primary"       — this file IS the main thing being worked on
    - "supplementary" — this file provides reference context or background
    - "raw_data"      — this file is raw data to be processed as pipeline input

    Return ONLY valid JSON array:
    [{{
      "file_path": "...",
      "graph_id": 0,
      "role": "primary|supplementary|raw_data",
      "reasoning": "brief"
    }}]"#,
            text_intent_summary,
            serde_json::to_string_pretty(&file_summaries).unwrap_or_default()
        );

        let input = serde_json::json!({
            "prompt": prompt,
            "max_tokens": 500,
            "temperature": 0.1,
            "system_context": "File role classification. Return only valid JSON array."
        });

        let result = self.metered_execute(state, 9, input).await.unwrap_or_default();
        let raw = result
            .get("response")
            .and_then(|r| r.as_str())
            .unwrap_or("[]");
        let json_str = Self::extract_json_from_response(raw, '[', ']');

        let classifications: Vec<ClassifiedFileGraph> =
            serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|v| {
                    Some(ClassifiedFileGraph {
                        file_path: v["file_path"].as_str()?.to_string(),
                        graph_id: v["graph_id"].as_u64().or_else(|| {
                            state
                                .file_graphs
                                .get(v["file_path"].as_str().unwrap_or(""))
                                .copied()
                        })?,
                        modality: self.detect_file_modality(v["file_path"].as_str().unwrap_or("")),
                        role: match v["role"].as_str().unwrap_or("raw_data") {
                            "primary" => FileGraphRole::Primary,
                            "supplementary" => FileGraphRole::Supplementary,
                            _ => FileGraphRole::RawData,
                        },
                        reasoning: v["reasoning"].as_str().unwrap_or("").to_string(),
                    })
                })
                .collect();

        state.classified_file_graphs = classifications;
        Ok(())
    }

    /// File graphs are created FIRST, before any classification.
    /// MUST be called before classify_file_graphs_post_creation.
    /// Analyze text via a modality pipeline. Returns the analysis result.
    /// Callers handle graph creation and storage separately.
    pub(crate) async fn process_modality(
        &self,
        text: &str,
        pipeline_id: u64,
        available_modalities: &[String],
        processing_path: Option<ProcessingPathPref>,
        executor_model: Option<ExecutorModelKind>,
        max_chunk_tokens: Option<u32>,
    ) -> Result<serde_json::Value, String> {
        let mut action = serde_json::json!({
            "type": "Analyze",
            "text": text,
            "depth": "Standard",
            "extract_entities": true,
            "extract_topics": true,
            "extract_structure": false,
            "available_modalities": available_modalities
        });
        if let Some(p) = processing_path {
            action["processing_path"] = serde_json::json!(path_str(p));
        }
        if let Some(e) = executor_model {
            action["executor_model"] = serde_json::json!(executor_model_str(e));
        }
        if let Some(m) = max_chunk_tokens {
            action["max_chunk_tokens"] = serde_json::json!(m);
        }
        self.executor
            .execute(pipeline_id, serde_json::json!({ "action": action }))
            .await
    }

    /// Detect modality string from a file path using extension and path hints.
    pub(crate) fn detect_file_modality(&self, file_path: &str) -> String {
        let path_lower = file_path.to_lowercase();
        // Path hints (highest priority for ambiguous extensions)
        if path_lower.contains("radar") || path_lower.contains("sar") {
            return "radar".to_string();
        }
        if path_lower.contains("sonar") || path_lower.contains("bathymet") {
            return "sonar".to_string();
        }
        if path_lower.contains("thermal") || path_lower.contains("infrared") {
            return "thermal".to_string();
        }
        if path_lower.contains("hyperspectral") || path_lower.contains("multispectral") {
            return "hyperspectral".to_string();
        }
        if path_lower.contains("imu") || path_lower.contains("accelerom") {
            return "imu".to_string();
        }

        // Extension-based detection
        let ext = std::path::Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "rs" | "py" | "js" | "ts" | "go" | "java" | "cpp" | "c" | "rb" | "swift" | "kt"
            | "sql" | "yaml" | "yml" | "toml" | "json" | "xml" => "code".to_string(),
            "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "tiff" | "svg" => "image".to_string(),
            "mp3" | "wav" | "flac" | "ogg" | "aac" | "m4a" => "audio".to_string(),
            "mp4" | "mkv" | "avi" | "mov" | "webm" => "video".to_string(),
            "blend" | "obj" | "glb" | "gltf" | "fbx" | "stl" | "usd" | "urdf" => "3d".to_string(),
            "pdb" | "mol" | "sdf" | "cif" => "chemistry".to_string(),
            "fasta" | "fa" | "fastq" | "vcf" | "gff" | "bam" | "sam" => "dna".to_string(),
            "edf" | "bdf" | "gdf" | "fif" => "eeg".to_string(),
            "geojson" | "kml" | "shp" | "gpx" => "geospatial".to_string(),
            "step" | "stp" | "iges" | "brep" => "parametric_cad".to_string(),
            "pcd" | "las" | "laz" => "depth".to_string(),
            "xtf" | "jsf" | "s7k" | "gsf" => "sonar".to_string(),
            "iq" | "cphd" | "sicd" | "nitf" => "radar".to_string(),
            "hdr" | "bil" | "bip" | "bsq" | "h4" => "hyperspectral".to_string(),
            "tex" | "latex" | "nb" => "math".to_string(),
            "txt" | "md" | "rst" | "pdf" | "docx" | "doc" => "text".to_string(),
            _ => "unknown".to_string(),
        }
    }

    /// Map modality name string to pipeline ID.
    pub(crate) fn modality_name_to_pipeline_id(&self, modality: &str) -> u64 {
        match modality {
            "text" => 100,
            "code" => 101,
            "image" => 102,
            "audio" => 103,
            "video" => 104,
            "math" => 105,
            "chemistry" => 106,
            "dna" => 107,
            "eeg" => 108,
            "3d" => 109,
            "sound" => 110,
            "biology" => 111,
            "proteomics" => 112,
            "haptic" => 113,
            "thermal" => 114,
            "depth" => 115,
            "imu" => 116,
            "geospatial" => 117,
            "electromagnetic" => 118,
            "bci" => 119,
            "parametric_cad" => 120,
            "kinematics" => 121,
            "control_systems" => 122,
            "network_topology" => 123,
            "radar" => 124,
            "sonar" => 125,
            "hyperspectral" => 126,
            _ => 0,
        }
    }
}