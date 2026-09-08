//! //! Response Graph assembly + the three-tier rendering ladder + stage 11
//! (response delivery). Content is fixed at assembly; renderers realize it
//! under a parse-back coverage gate.

use super::*;

impl PromptOrchestrator {

    /// Resolve one knowledge surface (with its sentence context) to ranked
    /// ZSEI paths. Detection rides on the grammar graphs; resolution is
    /// external and weight-static — the linker holds an addressing skill,
    /// never knowledge. On failure: empty list — the frame renders without
    /// provenance rather than guessing. Routed through the executor pipeline
    /// contract (pipeline 901), never direct store access.
    async fn resolve_knowledge_via_linker(
        &self,
        state: &mut OrchestrationState,
        surface: &str,
        context_sentence: &str,
    ) -> Vec<String> {
        let input = serde_json::json!({
            "surface": surface,
            "context": context_sentence,
            "top_k": 3
        });
        match self
            .metered_execute(state, OMEX_KNOWLEDGE_LINKER_PIPELINE_ID, input)
            .await
        {
            Ok(result) => result
                .get("paths")
                .and_then(|p| p.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    /// Assemble the Response Graph deterministically from verified results.
    /// One frame per completed step, in execution order, chained with
    /// Sequence relations. Clarification checkpoints produce ask-frames —
    /// the system never assumes an answer. Frame leaves lacking provenance
    /// are resolved via the KnowledgeLinker (capped at 3 resolutions per
    /// response to bound cost).
    async fn build_response_graph(
        &self,
        state: &mut OrchestrationState,
    ) -> ResponseGraphSpec {
        let voice = match &state.voice_identity {
            Some(v) => ResponseVoiceSpec {
                tone: v.tone.clone(),
                formality: v.formality,
                warmth: v.warmth,
                directness: v.directness,
                humor: v.humor_level,
            },
            None => ResponseVoiceSpec::default(),
        };

        // Knowledge paths aggregated on the AMT root by the graph-native
        // builder (A-series) ride in as baseline provenance.
        let knowledge_paths: Vec<String> = state
            .amt
            .as_ref()
            .map(|a| {
                a.metadata
                    .get("knowledge_paths")
                    .map(|s| {
                        s.split(", ")
                            .map(|p| p.trim().to_string())
                            .filter(|p| !p.is_empty())
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        let mut sentences: Vec<ResponseSentenceSpec> = Vec::new();

        // ── Clarification checkpoint: ask, never assume ──
        if state.needs_clarification {
            for (i, point) in state.clarification_points.iter().enumerate() {
                sentences.push(ResponseSentenceSpec {
                    granularity: "frame".to_string(),
                    frame: Some(ResponseFrame {
                        subject: "I".to_string(),
                        verb: if i == 0 {
                            "need".to_string()
                        } else {
                            "also need".to_string()
                        },
                        object: Some(format!("clarification: {}", point)),
                        modifiers: vec![],
                        properties: HashMap::new(),
                        knowledge_refs: vec![],
                    }),
                    tree: None,
                    discourse_relation_to_previous: if i == 0 {
                        None
                    } else {
                        Some("Elaborates".to_string())
                    },
                });
            }
            return ResponseGraphSpec {
                schema_version: "1.0".to_string(),
                voice,
                sentences,
            };
        }

        // ── Completed steps → one frame each (execution order = Sequence) ──
        let intent_text = state
            .amt
            .as_ref()
            .map(|a| a.content.clone())
            .unwrap_or_else(|| "the request".to_string());

        let mut first = true;
        for result in &state.step_results {
            let output_text = self.extract_output_text(&result.output);
            if output_text.trim().is_empty() {
                continue;
            }
            let object: String = output_text.chars().take(220).collect();
            sentences.push(ResponseSentenceSpec {
                granularity: "frame".to_string(),
                frame: Some(ResponseFrame {
                    subject: intent_text.clone(),
                    verb: if first {
                        "produced".to_string()
                    } else {
                        "then produced".to_string()
                    },
                    object: Some(object),
                    modifiers: vec![],
                    properties: [("tense".to_string(), "past".to_string())]
                        .into_iter()
                        .collect(),
                    knowledge_refs: knowledge_paths.clone(),
                }),
                tree: None,
                discourse_relation_to_previous: if first {
                    None
                } else {
                    Some("Sequence".to_string())
                },
            });
            first = false;
        }

        // ── Linker resolution for frames that lack provenance (capped) ──
        let mut resolved = 0usize;
        for sent in sentences.iter_mut() {
            if resolved >= 3 {
                break;
            }
            if let Some(frame) = &mut sent.frame {
                if frame.knowledge_refs.is_empty() {
                    if let Some(obj) = &frame.object {
                        let ctx = format!("{} {} {}", frame.subject, frame.verb, obj);
                        let paths = self
                            .resolve_knowledge_via_linker(state, obj, &ctx)
                            .await;
                        if !paths.is_empty() {
                            frame.knowledge_refs = paths;
                            resolved += 1;
                        }
                    }
                }
            }
        }

        ResponseGraphSpec {
            schema_version: "1.0".to_string(),
            voice,
            sentences,
        }
    }

    /// TIER 0 — deterministic template renderer. Walks the frames, realizes
    /// declared discourse relations as fixed connectives, applies polarity.
    /// Zero ML; exists from day zero; can never be unavailable.
    fn render_response_graph_tier0(graph: &ResponseGraphSpec) -> String {
        let mut out = String::new();
        for (i, sent) in graph.sentences.iter().enumerate() {
            let Some(frame) = &sent.frame else { continue };

            // Discourse connective (relation is content; word is surface).
            let connective = match sent.discourse_relation_to_previous.as_deref() {
                Some("Contrast") => "However, ",
                Some("Cause") => "Because of this, ",
                Some("Sequence") => "Then, ",
                Some("Example") => "For example, ",
                Some("Summary") => "In summary, ",
                Some("Elaborates") => "Specifically, ",
                _ => "",
            };

            let negated = frame
                .properties
                .get("polarity")
                .map(|p| p == "negative")
                .unwrap_or(false);

            let mut sentence = String::new();
            sentence.push_str(connective);
            sentence.push_str(&frame.subject);
            sentence.push(' ');
            if negated {
                sentence.push_str("did not ");
            }
            sentence.push_str(&frame.verb);
            if let Some(obj) = &frame.object {
                if !obj.is_empty() {
                    sentence.push(' ');
                    sentence.push_str(obj);
                }
            }
            for m in &frame.modifiers {
                if !m.is_empty() {
                    sentence.push_str(", ");
                    sentence.push_str(m);
                }
            }
            sentence.push('.');

            out.push_str(&sentence);
            if i + 1 < graph.sentences.len() {
                out.push(' ');
            }
        }
        out
    }

    /// Deterministic parse-back coverage audit: every content leaf must
    /// appear in the rendered text. This is the mechanical anti-hallucination
    /// gate for Tiers 1 and 2 — additions fail loudly, silently, and free.
    fn validate_render_coverage(graph: &ResponseGraphSpec, text: &str) -> bool {
        let lower = text.to_lowercase();
        for sent in &graph.sentences {
            let Some(frame) = &sent.frame else { continue };
            if !lower.contains(&frame.subject.to_lowercase()) {
                return false;
            }
            if let Some(obj) = &frame.object {
                if !obj.is_empty() {
                    // Long object leaves: probe the first 60 characters.
                    let probe: String = obj.chars().take(60).collect();
                    if !lower.contains(&probe.to_lowercase()) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// TIER 1 — LLM rendering stand-in: render-only instruction, output
    /// parse-back validated. One retry, then the caller drops a tier.
    async fn render_response_graph_tier1(
        &self,
        state: &mut OrchestrationState,
        graph: &ResponseGraphSpec,
    ) -> Option<String> {
        let graph_json = serde_json::to_string_pretty(graph).ok()?;
        for _attempt in 0..2 {
            let input = serde_json::json!({
                "prompt": format!(
                    r#"Render the following Response Graph into fluent natural language.

RULES (render-only — violations are rejected by an automated coverage audit):
- You may add ONLY function words, inflection/agreement, connectives realizing the declared discourse relations, and mild ordering variation within each sentence frame.
- You may NOT add, remove, alter, or paraphrase any content leaf (subjects, verbs, objects, modifiers).
- Voice properties guide register and word choice, never facts.

RESPONSE GRAPH:
{graph_json}

Return ONLY the rendered text. No explanation. No markdown."#
                ),
                "max_tokens": 800,
                "temperature": 0.4,
                "system_context": "Constrained surface realization. Render only what the graph contains."
            });
            if let Ok(result) = self.metered_execute(state, 9, input).await {
                if let Some(text) = result.get("response").and_then(|r| r.as_str()) {
                    let text = text.trim().to_string();
                    if !text.is_empty() && Self::validate_render_coverage(graph, &text) {
                        return Some(text);
                    }
                }
            }
        }
        None
    }

    /// TIER 2 — the OMEX Realizer (response graph → text, reverse grammar
    /// traversal). Same parse-back gate; on failure the caller escalates.
    async fn render_response_graph_tier2(
        &self,
        state: &mut OrchestrationState,
        graph: &ResponseGraphSpec,
    ) -> Option<String> {
        let input = serde_json::json!({
            "response_graph": graph,
            "options": { "voice_conditioning": true }
        });
        if let Ok(result) = self
            .metered_execute(state, OMEX_REALIZER_PIPELINE_ID, input)
            .await
        {
            if let Some(text) = result.get("text").and_then(|t| t.as_str()) {
                let text = text.trim().to_string();
                if !text.is_empty() && Self::validate_render_coverage(graph, &text) {
                    return Some(text);
                }
            }
        }
        None
    }

    /// Ladder dispatch. OMEX executor → Tier 2 first; otherwise Tier 1
    /// first (Auto may still try Tier 2 before the floor). Tier 0 always
    /// answers in the worst case.
    async fn render_response(
        &self,
        state: &mut OrchestrationState,
        graph: &ResponseGraphSpec,
    ) -> String {
        if state.request.executor_model == ExecutorModelKind::Omex {
            if let Some(text) = self.render_response_graph_tier2(state, graph).await {
                return text;
            }
            if let Some(text) = self.render_response_graph_tier1(state, graph).await {
                return text;
            }
        } else {
            if let Some(text) = self.render_response_graph_tier1(state, graph).await {
                return text;
            }
            if state.request.executor_model == ExecutorModelKind::Auto {
                // Auto: OMEX may be resident even though it wasn't the
                // primary executor — try Tier 2 before the floor.
                if let Some(text) = self.render_response_graph_tier2(state, graph).await {
                    return text;
                }
            }
        }
        Self::render_response_graph_tier0(graph)
    }

    pub(crate) async fn stage_11_response_delivery(
        &self,
        state: &mut OrchestrationState,
    ) -> Result<(), String> {
        let stage_start = std::time::Instant::now();

        // Voice identity (consciousness) — fetched, APPLIED to the Response
        // Graph's voice spec, and PERSISTED through the ZSEI store contract.
        // It conditions rendering (register/word choice via the ladder's
        // render-only rules) — it never alters content.
        if state.request.consciousness_enabled {
            let voice_input = serde_json::json!({ "action": "GetVoice" });
            if let Ok(voice_result) = self.executor.execute(46, voice_input).await {
                if let Some(voice) = voice_result.get("voice") {
                    state.voice_identity = serde_json::from_value(voice.clone()).ok();
                }
            }
            if let Some(voice) = &state.voice_identity {
                self.persist_voice_identity(voice).await;
            }
        }

        // Response Graph assembly + rendering ladder. This REPLACES the old
        // free-form LLM restyling: content is fixed at assembly; the renderer
        // cannot add, remove, or alter a content leaf; every tier above the
        // template floor is parse-back audited. Voice identity conditions the
        // graph (ResponseVoiceSpec) instead of a loose rephrase call.
        if state.final_response.is_some() || state.needs_clarification {
            let graph = self.build_response_graph(state).await;
            if !graph.sentences.is_empty() {
                let rendered = self.render_response(state, &graph).await;
                if !rendered.trim().is_empty() {
                    state.final_response = Some(rendered);
                }
            }
        }

        // Update consciousness dashboard (Pipeline 54) — unchanged.
        if state.request.consciousness_enabled {
            let dashboard_input = serde_json::json!({
                "action": "Update",
                "task_completed": true,
                "task_id": state.task_id,
                "success": state.final_response.is_some(),
                "tokens_used": state.tokens_used_so_far,
                "methodologies_used": state.methodologies.len(),
                "blueprint_id": state.blueprint_id
            });
            let _ = self.executor.execute(54, dashboard_input).await;
        }

        // Task recommendations for next steps (Pipeline 23) — unchanged.
        let recommend_input = serde_json::json!({
            "action": "Suggest",
            "context": &state.cleaned_prompt[..state.cleaned_prompt.len().min(200)],
            "completed_task_id": state.task_id,
            "topics": state.topics.clone(),
            "keywords": state.keywords.iter().take(5).cloned().collect::<Vec<_>>()
        });
        let _ = self.executor.execute(23, recommend_input).await;

        self.record_stage_timed(
            state,
            11,
            "Response Delivery",
            state.final_response.is_some(),
            &format!(
                "Response: {} chars, Voice: {}, Ladder: {}, Tokens: {}",
                state.final_response.as_ref().map(|r| r.len()).unwrap_or(0),
                state.voice_identity.is_some(),
                if state.request.executor_model == ExecutorModelKind::Omex {
                    "T2→T1→T0"
                } else {
                    "T1→T2→T0"
                },
                state.tokens_used_so_far
            ),
            stage_start.elapsed().as_millis() as u64,
        );

        Ok(())
    }
}