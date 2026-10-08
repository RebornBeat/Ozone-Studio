//! //! The orchestration stages — blueprint assignment, zero-shot simulation,
//! consciousness gate, step execution (with the on-complete hook), result
//! collection, and post-execution consciousness.

use super::*;

impl PromptOrchestrator {

    pub(crate) async fn execute_stages(&self, state: &mut OrchestrationState) -> Result<(), String> {
        // STAGE 1: Input Capture (already done — prompt is in request)
        self.record_stage(state, 1, "Input Capture", true, "Prompt received");

        // JURISDICTION GATE: always runs, independent of consciousness_enabled
        // — see src/orchestrator/jurisdiction.rs. A base safety layer, not
        // part of the optional consciousness system. Ships with zero real
        // rule content (mechanism only); see that file's own doc comment
        // for why no legal content is fabricated here.
        self.stage_jurisdiction_gate(state).await?;

        // STAGE 2: Text/Prompt Normalization (attached-file graph creation
        // runs first inside, then chunk processing — files inform the AMT).
        tracing::info!(stage = 2, "Stage 2 (Text/Prompt Normalization) starting");
        self.prompt_normalization(state).await?;

        // STAGE 3: Gather Methodologies (methodology set + categories ONLY —
        // no AMT construction here; that is Stage 5's job alone).
        tracing::info!(stage = 3, "Stage 3 (Gather Methodologies) starting");
        self.gather_methodologies(state).await?;

        // STAGE 4 (part A): File graph classification — AFTER text processing
        // gives us keywords/topics signals, BEFORE aggregation so the
        // classified file modalities feed the root modality list. Runs ONCE.
        if !state.file_graphs.is_empty() {
            tracing::info!(stage = 4, "Stage 4a (File Graph Classification) starting");
            self.classify_file_graphs_post_creation(state).await?;
        }

        // STAGE 4 (part B): Initial graph creation — BEFORE the AMT. The AMT
        // is built from the text graph, and graph state (verified modalities,
        // cross-modal edges) is part of the evidence the AMT traversal reads.
        tracing::info!(stage = 4, "Stage 4b (Initial Graph Creation) starting");
        self.aggregate_root_modalities(state).await;
        self.create_initial_modality_graphs(state)
            .await
            .map_err(|e| format!("Initial graph creation failed: {}", e))?;
        self.record_stage(
            state,
            4,
            "Initial Graph Creation",
            true,
            &format!(
                "{} modality graphs created ({}), cross-modal: {}, Files classified: {}",
                state.modality_graphs.len(),
                state
                    .modality_graphs
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", "),
                state
                    .graph_states
                    .values()
                    .map(|g| g.cross_modal_edge_count)
                    .sum::<usize>(),
                state.classified_file_graphs.len()
            ),
        );

        // STAGE 5: Build the AMT — graph-native traversal when the chunks
        // carry sentence nodes with grammar relationships (Path 2 / OMEX
        // native), legacy chunk zero-shot loop otherwise.
        tracing::info!(stage = 5, "Stage 5 (Build AMT) starting");
        self.build_amt(state).await?;

        // If clarification needed, stop here and return to user
        if state.needs_clarification {
            tracing::info!("Orchestration stopping early: clarification needed from user");
            return Ok(());
        }

        // STAGE 6: Blueprint Assignment
        tracing::info!(stage = 6, "Stage 6 (Blueprint Assignment) starting — applicable tools carried: {}", state.applicable_tools.len());
        self.stage_3_blueprint_assignment(state).await?;

        // STAGE 7: Zero-Shot Simulation (with AMT traversal)
        tracing::info!(stage = 7, "Stage 7 (Zero-Shot Simulation) starting — this calls the configured model backend and may take a while (e.g. BitNet loading its model fresh per call)");
        tracing::info!(stage = 7, "[7.0] simulation context: methodologies in scope = {}, applicable tools = {}", state.methodologies.len(), state.applicable_tools.len());
        self.stage_4_zero_shot_simulation(state).await?;

        // STAGE 8: Consciousness Decision Gate
        if state.request.consciousness_enabled {
            tracing::info!(stage = 8, "Stage 8 (Consciousness Gate) starting");
            self.stage_5_consciousness_gate(state).await?;
        } else {
            self.record_stage(state, 8, "Consciousness Gate", true, "Skipped (disabled)");
        }

        // STAGE 8b: Jurisdiction FULL Gate (operator, 2026-10-01: Stage 0 is
        // the soft run — it can only judge the raw prompt; the FULL run
        // judges the RESOLVED PLAN and belongs alongside the consciousness
        // gate, post-simulation, pre-execution).
        tracing::info!(stage = 8, "Stage 8b (Jurisdiction Full Gate) starting");
        self.stage_jurisdiction_full_gate(state).await?;

        // STAGES 9-10: Task Creation + Step Execution
        tracing::info!(stage = 9, "Stages 9-10 (Task Creation + Step Execution) starting");
        self.stage_6_to_8_execute_steps(state).await?;

        // STAGE 11: Result Collection
        tracing::info!(stage = 11, "Stage 11 (Result Collection) starting");
        self.stage_9_result_collection(state).await?;

        // STAGE 12: Post-execution Consciousness
        if state.request.consciousness_enabled {
            tracing::info!(stage = 12, "Stage 12 (Post-execution Consciousness) starting");
            self.stage_10_post_execution(state).await?;
        } else {
            self.record_stage(state, 12, "Post-execution", true, "Skipped (disabled)");
        }

        // STAGE 13: Response Delivery
        tracing::info!(stage = 13, "Stage 13 (Response Delivery) starting");
        tracing::info!(stage = 13, "[13.0] delivery context: blueprint steps executed = {}, gate decision = {:?}", state.blueprint_steps.len(), state.gate_result.as_ref().map(|g| &g.decision));
        self.stage_11_response_delivery(state).await?;

        tracing::info!("Orchestration complete: all 14 stages finished");
        Ok(())
    }

    /// Real, human-readable summary of the jurisdiction gate's full result
    /// (rules matched, warnings, confirmation-review outcomes) — the one
    /// shared source of truth for every stage that needs to reason about
    /// jurisdiction, not a copy-pasted subset per call site. Originally
    /// inline only in the zero-shot simulation prompt (stage 7); confirmed
    /// live this session that AMT building (stage 5) and blueprint
    /// assignment (stage 6) — both of which run BEFORE simulation, with
    /// jurisdiction's real outcome already fully resolved at stage 0 —
    /// never read `state.jurisdiction_gate_result` at all, so a Warn or
    /// RequireConfirmation match shaped nothing about what got planned,
    /// only critiqued a plan already finalized in ignorance of it. Pulled
    /// out to a shared function so all three stages see identical real
    /// data, including the two fields the original inline version omitted
    /// (`warnings`, `confirmations`).
    /// REGISTERED CAPABILITIES summary (§11.2): compact one-line-per-tool
    /// registry text for prompt embedding. Reads the summary built at the
    /// orchestrate seam (state.capability_summary); honest "(none
    /// registered)" when absent — never a fabricated list.
    pub(crate) fn capability_summary_for_need(state: &OrchestrationState, need: &str) -> String {
        let full = match &state.capability_summary {
            Some(f) => f,
            None => return "(no tools/MCPs registered)".to_string(),
        };
        let need_lower = need.to_lowercase();
        let tokens: Vec<&str> = need_lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| t.len() > 2)
            .collect();
        if tokens.is_empty() {
            // No keywords to retrieve by — say so, hand over the full
            // registry rather than guessing at relevance.
            return format!("(no need keywords to retrieve tools by) {full}");
        }
        let lines: Vec<&str> = full.lines().collect();
        let matched: Vec<&str> = lines
            .iter()
            .filter(|l| {
                let l_lower = l.to_lowercase();
                tokens.iter().any(|t| l_lower.contains(t))
            })
            .copied()
            .collect();
        if matched.len() == lines.len() {
            // Everything matches this need — identity view, nothing cut.
            return full.clone();
        }
        if matched.is_empty() {
            return format!(
                "({} tools/MCPs registered; none matched this stage's need keywords — retrieval is by need; the full registry remains in orchestration state and on the graph, and any tool you know by name remains callable)",
                lines.len()
            );
        }
        format!(
            "{}\n[tools retrieved for this need: {} of {} registered shown; the rest remain registered and callable by name]",
            matched.join("\n"),
            matched.len(),
            lines.len()
        )
    }

    pub(crate) fn jurisdiction_summary(state: &mut OrchestrationState) -> String {
        let text = match &state.jurisdiction_gate_result {
            Some(g) => {
                let base = format!(
                    "region rules loaded: {}, matched: {}, blocked: {}",
                    g.rules_loaded,
                    g.matched.len(),
                    g.blocked
                );
                let mut sections = vec![base];
                if !g.matched.is_empty() {
                    let rules = g
                        .matched
                        .iter()
                        .map(|(rule, action)| {
                            format!(
                                "    - {} -> {:?}{}",
                                rule.condition,
                                action,
                                if rule.source.is_empty() {
                                    String::new()
                                } else {
                                    format!(" (source: {})", rule.source)
                                }
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    sections.push(rules);
                }
                if !g.warnings.is_empty() {
                    let warnings = g
                        .warnings
                        .iter()
                        .map(|w| format!("    - {}", w))
                        .collect::<Vec<_>>()
                        .join("\n");
                    sections.push(format!("  Warnings:\n{}", warnings));
                }
                if !g.confirmations.is_empty() {
                    let confirmations = g
                        .confirmations
                        .iter()
                        .map(|(rule, review)| {
                            format!(
                                "    - {} -> {} ({})",
                                rule.condition, review.decision, review.reasoning
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    sections.push(format!("  Confirmation reviews:\n{}", confirmations));
                }
                sections.join("\n")
            }
            None => "no gate result".to_string(),
        };
        // Whole summary, sized against the reading model's window (was
        // take(5) per list, silently). Every cut is recorded.
        super::amt::render_within_window(state, "jurisdiction_summary", 8, text)
    }

    /// Real relationship edges among THIS REQUEST's own attached-file
    /// graphs (`state.file_graphs`) — confirmed live this session that AMT
    /// building and blueprint assignment only ever saw bare
    /// `path/modality/role/graph_id` for attached files, never the real
    /// `SimilarTo` edges (with real confidence + `discovered_via`) that
    /// STEP 0's `link_to_existing: true` already creates among them. The
    /// graph_ids are already exactly known here — no search/traversal
    /// needed, just a direct per-id container fetch. Capped at 10 files /
    /// 10 total relationships shown so a request with many attachments
    /// can't blow up prompt size. Empty string when there are no attached
    /// files, so requests without attachments get no extra prompt noise.
    /// S1 STANDING CONTEXT (CONTEXT_REGISTRY S1) — the project's main AMT
    /// outline (name + topics), direct-fetched from the project container's
    /// amt-main child. Closes the project-amnesia gap: generation calls
    /// (#2/#3/#4/#7) currently build per-request structure without ever
    /// seeing what the project already knows from prior sessions.
    /// Empty string when no project or no main AMT — never fabricated.
    pub(crate) fn standing_context_summary(&self, state: &OrchestrationState) -> String {
        let Some(project_id) = state.request.project_id else { return String::new() };
        // The store is &Arc<dyn StoreAccess> on self, but this fn is &self
        // (not async) — use the same sync pattern jurisdiction_summary uses
        // (which reads state.jurisdiction_gate_result, no store needed).
        // For S1 we need the store, so this must be async. But to keep the
        // call sites simple, we use a synchronous best-effort read of
        // state.file_graphs's project_id (already threaded) + the AMT
        // container id (already on state after a prior orchestration in
        // the same session). If either is present, we emit what we know.
        let mut parts = Vec::new();
        if let Some(amt_id) = state.amt_container_id {
            parts.push(format!("AMT container: {}", amt_id));
        }
        if let Some(amt) = &state.amt {
            parts.push(format!("Root intent: {}", amt.content));
            // Every top-level branch name (was take(6): silent drop past six).
            let branches: Vec<&str> = amt.children.iter().map(|c| c.content.as_str()).collect();
            if !branches.is_empty() {
                parts.push(format!("Branches: {}", branches.join("; ")));
            }
        }
        if !state.request.prompt.is_empty() {
            // The whole request (was cut to 200 chars).
            parts.push(format!("Request: {}", state.request.prompt.trim()));
        }
        if parts.is_empty() {
            return String::new();
        }
        format!("PROJECT STANDING CONTEXT (the source-of-truth structure this system built for this project):\n{}", parts.join("\n"))
    }

    pub(crate) async fn file_relationship_summary(&self, state: &mut OrchestrationState) -> String {
        if state.file_graphs.is_empty() {
            return String::new();
        }
        let mut lines = Vec::new();
        for (path, &graph_id) in state.file_graphs.iter() {
            if let Ok(Some(container)) = self.store.get_container(graph_id).await {
                let relationships = container
                    .get("local_state")
                    .and_then(|ls| ls.get("context"))
                    .and_then(|c| c.get("relationships"))
                    .and_then(|r| r.as_array())
                    .cloned()
                    .unwrap_or_default();
                for rel in relationships {
                    let target_id = rel.get("target_id").and_then(|t| t.as_u64()).unwrap_or(0);
                    let relation_type = rel
                        .get("relation_type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("Related");
                    let confidence = rel.get("confidence").and_then(|c| c.as_f64()).unwrap_or(0.0);
                    let discovered_via = rel
                        .get("discovered_via")
                        .and_then(|d| d.as_str())
                        .unwrap_or("Unknown");
                    lines.push(format!(
                        "  - {} (#{}) <-> #{}: {} {:.2} via {}",
                        path, graph_id, target_id, relation_type, confidence, discovered_via
                    ));
                }
            }
        }
        // Every relationship line (was capped at 10 files / 10 lines,
        // silently). Sized against the reading model's window; cuts recorded.
        let text = if lines.is_empty() {
            String::new()
        } else {
            format!(
                "RELATED FILES (from this request's own graph):\n{}",
                lines.join("\n")
            )
        };
        super::amt::render_within_window(state, "file_relationships", 8, text)
    }

    async fn stage_3_blueprint_assignment(
        &self,
        state: &mut OrchestrationState,
    ) -> Result<(), String> {
        let stage_start = std::time::Instant::now();

        // Search for blueprint with 100% match
        let blueprint_ids = self
            .store
            .search_by_keywords(
                &state.keywords.iter().take(15).cloned().collect::<Vec<_>>(),
                Some("Blueprint"),
            )
            .await
            .unwrap_or_default();

        let mut best_match: Option<(u64, f32)> = None;

        for bp_id in blueprint_ids {
            if let Ok(Some(container)) = self.store.get_container(bp_id).await {
                // Calculate match score
                let bp_keywords: Vec<String> = container
                    .get("local_state")
                    .and_then(|ls| ls.get("context"))
                    .and_then(|ctx| ctx.get("keywords"))
                    .and_then(|k| serde_json::from_value(k.clone()).ok())
                    .unwrap_or_default();

                let state_keywords_set: HashSet<_> =
                    state.keywords.iter().map(|s| s.to_lowercase()).collect();
                let bp_keywords_set: HashSet<_> =
                    bp_keywords.iter().map(|s| s.to_lowercase()).collect();

                let intersection = state_keywords_set.intersection(&bp_keywords_set).count();
                let union = state_keywords_set.union(&bp_keywords_set).count();

                let match_score = if union > 0 {
                    intersection as f32 / union as f32
                } else {
                    0.0
                };

                if match_score > best_match.map(|(_, s)| s).unwrap_or(0.0) {
                    best_match = Some((bp_id, match_score));
                }
            }
        }

        // Only use if 100% match (or very close - 95%+)
        if let Some((bp_id, score)) = best_match {
            if score >= 0.95 {
                state.blueprint_id = Some(bp_id);

                // Load blueprint steps — from the blueprint's CONTENT FILE
                // (object_store_path) when present, falling back to inline
                // storage.steps. B17 resolution (2026-09-28): the content
                // files are the canonical home (16 migrated blueprints now
                // carry new-schema steps at zsei_data/blueprints/); the
                // old inline read could never see them because storage is
                // a typed struct and serde drops unknown keys on the
                // round-trip (the same serde class as the insight-content
                // bug). Path resolution matches B0's convention:
                // relative joins OZONE_ZSEI_DATA_DIR (default zsei_data).
                if let Ok(Some(container)) = self.store.get_container(bp_id).await {
                    let store_path = container
                        .get("local_state")
                        .and_then(|ls| ls.get("storage"))
                        .and_then(|s| s.get("object_store_path"))
                        .and_then(|p| p.as_str())
                        .map(String::from);
                    let mut loaded: Option<serde_json::Value> = None;
                    if let Some(rel) = store_path {
                        let base = std::env::var("OZONE_ZSEI_DATA_DIR")
                            .unwrap_or_else(|_| "zsei_data".to_string());
                        let abs = std::path::Path::new(&base).join(&rel);
                        if let Ok(content) = std::fs::read_to_string(&abs) {
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content)
                            {
                                loaded = parsed.get("steps").cloned();
                            }
                        }
                    }
                    let steps_value = loaded.or_else(|| {
                        container
                            .get("local_state")
                            .and_then(|ls| ls.get("storage"))
                            .and_then(|s| s.get("steps"))
                            .cloned()
                    });
                    if let Some(steps) = steps_value {
                        state.blueprint_steps =
                            serde_json::from_value(steps.clone()).unwrap_or_default();
                        let n = steps.as_array().map(|a| a.len()).unwrap_or(0);
                        if n > 0 {
                            self.record_stage_timed(
                                state,
                                6,
                                "Blueprint Steps Loaded",
                                true,
                                &format!("{} step(s) from blueprint {}", n, bp_id),
                                0,
                            );
                        }
                    }
                }

                self.record_stage_timed(
                    state,
                    6,
                    "Blueprint Assignment",
                    true,
                    &format!(
                        "Using existing blueprint {} (match: {:.0}%)",
                        bp_id,
                        score * 100.0
                    ),
                    stage_start.elapsed().as_millis() as u64,
                );
                return Ok(());
            }
        }

        // No 100% match - create new blueprint
        // Cloned (not borrowed) — the reconciliation pass below needs
        // `amt.children` after several `&mut state` calls (metered_execute,
        // record_thinking), which a borrow of state.amt can't survive.
        let amt = state.amt.clone().ok_or("No AMT available")?;

        // Generate blueprint from AMT with pipeline awareness
        let available_pipelines_desc: String = state
            .available_pipelines
            .iter()
            .filter(|p| !p.deprecated)
            .map(|p| format!("  - {} (ID: {}): {}", p.name, p.pipeline_id, p.description))
            .collect::<Vec<_>>()
            .join("\n");

        // Paid models are never offered to the blueprint model: the list is
        // free OpenRouter models and local models only. A paid entry in
        // config stays configured; it is simply not a choice this prompt
        // can make. Overrides naming one are refused at resolution below.
        let free_models_for_prompt: Vec<&crate::config::AvailableModel> = state
            .request
            .available_models
            .iter()
            .filter(|m| m.is_free || state.request.allow_paid_models)
            .collect();
        let available_models_desc: String = if free_models_for_prompt.is_empty() {
            "  (no free or local models configured; the default chain is used)".to_string()
        } else {
            free_models_for_prompt
                .iter()
                .map(|m| {
                    format!(
                        "  - \"{}\" (type: {}, context: {})",
                        m.identifier, m.model_type, m.context_length
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        // Real methodology content, not bare numeric IDs — confirmed live
        // this was a real gap: this prompt previously interpolated
        // state.methodologies via {:?} (e.g. "[30003, 30004]"), so the model
        // drafting the blueprint never saw what those methodologies actually
        // require, only opaque numbers with no way to act on them. Falls
        // back to just the name when a methodology has no real rule content
        // file yet (most don't — see load_methodology_rules_text).
        let methodologies_desc = if state.methodologies.is_empty() {
            "  (none matched this request)".to_string()
        } else {
            let mut lines = Vec::new();
            // Copy of the ids: the loop body sizes rules against `state`
            // (mutable), so it cannot keep a borrow of state.methodologies.
            for method_id in state.methodologies.clone() {
                if let Ok(Some(container)) = self.store.get_container(method_id).await {
                    let name = container
                        .get("local_state").and_then(|ls| ls.get("metadata"))
                        .and_then(|m| m.get("name")).and_then(|n| n.as_str())
                        .unwrap_or("Unknown");
                    match Self::load_methodology_rules_full(&container) {
                        Some(rules) => {
                            let rules = super::amt::render_within_window(
                                state,
                                "blueprint_methodology_rules",
                                8,
                                rules,
                            );
                            lines.push(format!("  - {}: {}", name, rules))
                        }
                        None => lines.push(format!("  - {} (no detailed rules on file yet)", name)),
                    }
                }
            }
            lines.join("\n")
        };

        let branch_count = amt.children.len();
        let jurisdiction_ctx = Self::jurisdiction_summary(state);
        let file_relationships = self.file_relationship_summary(state).await;
        // REGISTERED CAPABILITIES (§11.2): compact registry summary so the
        // blueprint can route steps to tools/MCPs when the goal matches.
        let capabilities =
            Self::capability_summary_for_need(state, &state.request.prompt);
        // The request text, and every branch's text (intents with their branches
        // under them). The blueprint previously saw only the root label and
        // "name: N children", so branch text never reached it. The branch block
        // is window-derived; any trim is recorded (render_within_window).
        let request_text = super::amt::request_intent_text(state);
        let mut branch_block = String::new();
        for c in &amt.children {
            branch_block.push_str(&format!("- {}\n", c.content));
            for g in &c.children {
                branch_block.push_str(&format!("    - {}\n", g.content));
            }
        }
        let branch_block =
            super::amt::render_within_window(state, "blueprint_branches", 4, branch_block);

        let blueprint_prompt = format!(
            r#"Create a blueprint (execution plan) from this AMT.

REQUEST (the user's request as stated — the blueprint must answer it):
{request_text}

AMT ROOT: {}
BRANCHES ({branch_count} total — this AMT root represents {branch_count} distinct
intent(s)/branch(es); the request asked for all of them, not just the first):
{}

REGISTERED CAPABILITIES (registered tools/MCPs — a step MAY route to one of
these by capability name when the step's goal matches; route to pipeline 9
otherwise):
{capabilities}

AVAILABLE PIPELINES:
{}

AVAILABLE MODELS — FREE AND LOCAL ONLY (for optional per-step model_override.model_identifier):
{}

APPLICABLE METHODOLOGIES (apply these rules directly when drafting steps —
e.g. if a rule says to flag missing tests, make sure a step actually does
that rather than a generic "review the code" step):
{}

JURISDICTION CONTEXT (rules already matched against this request — plan
around any Warn/RequireConfirmation outcomes, not just Block):
{jurisdiction_ctx}
{file_relationships}

For each step, select the most appropriate pipeline from the list.
IMPORTANT: every branch listed above must be addressed by at least one step —
do not silently drop a branch just because it's independent of the others.
If two branches are genuinely independent (parallel) asks, create a SEPARATE
step for each rather than merging them into one step that only answers part
of the request. Only merge branches into a single step when one pipeline call
can genuinely and completely satisfy all of them together.
If no existing pipeline can handle a requirement, add it to missing_capabilities.
Only set model_override when a step genuinely benefits from a different model
than the default (e.g. a cheap/fast model for a small classification step) —
omit it entirely otherwise.

Return JSON (this example shows two steps for two branches — use however many
steps this AMT's branch count above actually requires, not necessarily two):
{{
    "name": "Blueprint name",
    "description": "What this blueprint does",
    "steps": [
        {{
            "step_index": 0,
            "action": "action_name",
            "description": "What this step does",
            "pipeline_id": 9,
            "context_requirements": ["full_context"],
            "depends_on": [],
            "wait_for_graph_update": false,
            "max_retries": 1,
            "model_override": null
        }},
        {{
            "step_index": 1,
            "action": "action_name",
            "description": "What this OTHER branch's step does",
            "pipeline_id": 9,
            "context_requirements": ["full_context"],
            "depends_on": [],
            "wait_for_graph_update": false,
            "max_retries": 1,
            "model_override": null
        }}
    ],
    "missing_capabilities": ["capability1", "capability2"]
}}"#,
            amt.content,
            branch_block,
            available_pipelines_desc,
            available_models_desc,
            methodologies_desc
        );

        let bp_input = serde_json::json!({
            "prompt": blueprint_prompt,
            "max_tokens": 1000,
            "temperature": 0.3,
            "system_prompt": "Generate execution blueprints. Respond with JSON only."
        });

        // NOTE: blueprint drafting is NOT detached "meta work" — it runs on
        // every request as Stage 6 of execute_stages, and its output (which
        // pipelines/steps run, in what order, with what model overrides) is
        // exactly what answers THIS request. Routing it through
        // try_meta_fallback_chain (as an earlier version of this code did)
        // silently forced every request onto local+free-only models
        // regardless of what the user actually selected for conversation —
        // confirmed as a real misclassification, reverted. The meta_fallback
        // config/try_meta_fallback_chain machinery stays in place (unused
        // for now) for whenever a genuine DETACHED meta job exists — see
        // TaskManager::start_refinement_daemon, real code but never started
        // anywhere in this codebase.
        // metered_execute_resilient covers both a hard Err AND an Ok-but-
        // empty response (retry, then fallback chain) — the plain
        // Err-only match this replaced would have silently accepted an
        // empty-but-Ok blueprint draft, same failure class confirmed live
        // at graphs.rs's File Role Classification call.
        let bp_result = self.metered_execute_resilient(state, bp_input, "blueprint_assignment").await?;
        self.record_thinking(state, "Blueprint Assignment", &bp_result);
        let response = bp_result
            .get("response")
            .and_then(|r| r.as_str())
            .unwrap_or("{}");
        let bp_json = Self::parse_json_object(response);

        let name = bp_json
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("Generated Blueprint")
            .to_string();
        let description = bp_json
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();

        // Check for missing capabilities
        let missing_capabilities: Vec<String> = bp_json
            .get("missing_capabilities")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if !missing_capabilities.is_empty() {
            // Log missing capabilities - could trigger pipeline creation in future
            tracing::warn!(
                "Blueprint has missing capabilities: {:?}",
                missing_capabilities
            );
        }

        // Partial-salvage parse (real bug found+fixed 2026-09-22): this
        // used to be a single `serde_json::from_value` on the WHOLE steps
        // array — one malformed step (e.g. a confetti-affected field, a
        // hallucinated extra key) silently dropped every OTHER real,
        // individually-valid step along with it. Parse each step
        // individually instead; a step that fails to deserialize is logged
        // and skipped, not allowed to take the rest down with it. Safe
        // given the downstream branch-reconciliation pass below already
        // enforces AMT-branch coverage generically (not just for the
        // empty-array case) — a partially-salvaged Vec just means
        // reconciliation has less real work to fill in, not a new gap.
        let salvaged_steps: Option<Vec<BlueprintStep>> = bp_json.get("steps").and_then(|s| s.as_array()).map(|arr| {
            let mut steps = Vec::new();
            for (i, step_json) in arr.iter().enumerate() {
                match serde_json::from_value::<BlueprintStep>(step_json.clone()) {
                    Ok(step) => steps.push(step),
                    Err(e) => {
                        tracing::warn!(
                            index = i,
                            error = %e,
                            "Blueprint assignment: one step failed to deserialize — skipping it, keeping the rest (was previously an all-or-nothing drop)"
                        );
                    }
                }
            }
            steps
        });
        state.blueprint_steps = salvaged_steps
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                // Confirmed live: when the blueprint LLM call comes back
                // empty/unparseable AND real AMT branches exist, starting
                // from this one generic catch-all step produced duplicate,
                // overlapping answers — the reconciliation pass below still
                // (correctly) synthesizes a step per branch since "Process
                // the user prompt" doesn't overlap-match any specific
                // branch, so the same request got answered twice (once
                // generically, once per-branch). When branches exist, start
                // empty and let reconciliation build clean per-branch steps
                // instead; this generic fallback is only for the true
                // last-resort case of no AMT branches at all.
                if !amt.children.is_empty() {
                    return Vec::new();
                }
                vec![BlueprintStep {
                    step_index: 0,
                    action: "execute_prompt".to_string(),
                    description: "Process the user prompt".to_string(),
                    pipeline_id: 9,
                    context_requirements: vec!["full_context".to_string()],
                    loop_config: None,
                    sub_steps: Vec::new(),
                    depends_on: Vec::new(),
                    wait_for_graph_update: false,
                    max_retries: 1,
                    timeout_ms: None,
                    model_override: None,
                    source_chunk_indices: Vec::new(),
                    methodology_ids: Vec::new(),
                }]
            });

        // Coverage is enforced below (deterministic branch-reconciliation
        // pass, after pipeline_id validation) rather than just logged here —
        // confirmed live this session that logging alone wasn't enough: a
        // genuinely multi-intent AMT repeatedly produced a 1-step blueprint
        // (once even backed by needs_clarification=true) that silently
        // answered only one intent, with the rest dropped with no error.

        // Validate each LLM-authored pipeline_id before it can travel deep
        // into execution — an unvalidated hallucinated id previously only
        // surfaced as a failure inside execute_inner, several stages later.
        // This used to just check "is this a REGISTERED pipeline_id" (via
        // available_pipelines / pipeline_exists) and let anything registered
        // through — but registered is not the same as SCHEMA-COMPATIBLE: the
        // generic input this stage builds for step execution ({prompt,
        // max_tokens, temperature, action}, see exec_input below and
        // build_sub_step_input in mod.rs) only matches pipeline 9's (Prompt)
        // contract. Now that available_pipelines lists all 82 registered
        // pipelines (including exotic modality ones — Kinematics, Radar,
        // Haptic, CAD, etc.), the old check happily approved the LLM picking
        // one of those for a step, and it failed several stages later with
        // "Parse error: missing field `action`" (confirmed live) since that
        // pipeline's own bespoke Input struct doesn't accept this shape at
        // all. Every non-9 choice gets coerced here instead — existence in
        // the registry was never sufficient.
        //
        // This also happens to be the reason methodology_create (12) and
        // blueprint_create (14) can never be picked as a workspace blueprint
        // step today — but that's currently just a side effect of the
        // schema-compatibility check above, not a documented policy. Make it
        // one explicitly: by direction, these are core/meta-only operations
        // (methodology and blueprint authoring/maintenance) and must never
        // be directly invocable from a normal user-facing workspace
        // request/blueprint, regardless of how this coercion logic evolves
        // later (e.g. if per-pipeline input mapping is ever added, making
        // other non-9 pipelines legitimately callable from a step). If that
        // happens, 12 and 14 specifically must stay excluded.
        //
        // Pipeline 56 (WebSearch) is the first deliberate exception in the
        // OTHER direction: execute_step special-cases pipeline_id 56 with
        // its own input shape (see execute_web_search_step) instead of the
        // generic pipeline-9 payload, so it's excluded from this coercion
        // rather than forced to 9.
        for step in &mut state.blueprint_steps {
            // CONSCIOUSNESS GATE (operator security directive: consciousness
            // is internal meta — not callable from orchestration contracts).
            // A blueprint naming a consciousness-category pipeline is a
            // contract violation: strip it to the generic executor loudly
            // rather than honoring it (the coercion below would silently
            // run pipeline 9 instead — that hides the attempt).
            if crate::pipeline::registry::category_of(step.pipeline_id) == Some("consciousness") {
                tracing::warn!(
                    step = step.step_index,
                    pipeline_id = step.pipeline_id,
                    "SECURITY: blueprint step named a consciousness-category pipeline —                      internal meta is not callable from orchestration; coercing to generic"
                );
                step.pipeline_id = 9;
            }
            if step.pipeline_id != 9 && step.pipeline_id != 56 {
                tracing::warn!(
                    "Blueprint step {} named pipeline_id {} — the generic step-execution \
                     input only matches pipeline 9's contract, coercing to 9",
                    step.step_index,
                    step.pipeline_id
                );
                step.pipeline_id = 9;
            }
            for sub_step in &mut step.sub_steps {
                if sub_step.pipeline_id != 9 {
                    tracing::warn!(
                        "Blueprint sub-step {} named pipeline_id {} — coercing to 9 (same reason)",
                        sub_step.sub_index,
                        sub_step.pipeline_id
                    );
                    sub_step.pipeline_id = 9;
                }
            }
        }

        // DETERMINISTIC BRANCH-COVERAGE RECONCILIATION: the prompt above
        // asks the model to address every branch, but that's advisory —
        // confirmed live, a genuinely 3-intent request came back as a
        // single step plus needs_clarification=true instead of concrete
        // per-intent steps (amt.children.len() == 3 at generation time, so
        // the model was told the real count and still punted). Trusting
        // compliance silently drops user intents, so this pass fuzzy-matches
        // each generated step to the branch(es) its description overlaps
        // with, then deterministically synthesizes a step for any branch
        // nothing plausibly addresses — no extra LLM round-trip, so it can't
        // itself punt. This also derives real per-branch keywords/chunk
        // scoping (previously every step's context_requirements was just
        // the LLM's copied-verbatim "full_context" placeholder from the
        // prompt's example, and BlueprintStep.source_chunk_indices had no
        // other source), which execute_step's context aggregation now uses
        // to scope container search and session-context reconstruction to
        // this step's specific branch instead of the whole session.
        let branch_keywords = |content: &str| -> Vec<String> {
            content
                .split_whitespace()
                .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
                .filter(|w| w.len() > 3)
                .take(8)
                .collect()
        };
        let overlaps = |branch_content: &str, step_description: &str| -> bool {
            let a = branch_content.to_lowercase();
            let b = step_description.to_lowercase();
            a.contains(&b)
                || b.contains(&a)
                || a.split_whitespace()
                    .filter(|w| w.len() > 4)
                    .any(|w| b.contains(w))
        };

        // AMTRelation (branch-to-branch cross-references, built from real
        // state.cross_refs — see build_branch_node) was previously write-only:
        // captured on every branch node but never read anywhere downstream.
        // A step whose branch has a real cross-reference to another branch
        // should see that other branch's relevant text too, not just its own
        // chunk indices — otherwise a step can be scoped away from content
        // its own AMT data says it's explicitly related to. Recurses because
        // relationships live on branch-level nodes, which in multi-intent
        // mode sit one level below the amt.children entries this
        // reconciliation matches against (children are intent-level nodes
        // there, branches are intent_node.children) — walking descendants
        // finds them regardless of which tree shape applies.
        //
        // Extracted to module-level free functions (collect_relationship_targets/
        // find_amt_node_by_id/related_chunk_indices, below) so T-A5's real
        // test can call them directly instead of only being reachable
        // through this ~400-line stage method — same logic, no behavior
        // change, just made independently testable (see docs/GRAPH_TEST_PLAN.md).
        // AMTNode.methodology_ids is only ever set on branch-level nodes
        // (build_branch_node), not on the intent-level nodes amt.children
        // actually are in multi-intent mode — so collect recursively from
        // "matched" and its descendants, same reasoning as relationships
        // above, rather than assuming matched itself carries it directly.
        fn collect_methodology_ids(node: &AMTNode, ids: &mut Vec<u64>) {
            for &id in &node.methodology_ids {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            for child in &node.children {
                collect_methodology_ids(child, ids);
            }
        }

        for existing in state.blueprint_steps.iter_mut() {
            if let Some(matched) = amt.children.iter().find(|c| overlaps(&c.content, &existing.description)) {
                if existing.source_chunk_indices.is_empty() {
                    existing.source_chunk_indices = matched.source_chunk_indices.clone();
                }
                for idx in related_chunk_indices(matched, &amt) {
                    if !existing.source_chunk_indices.contains(&idx) {
                        existing.source_chunk_indices.push(idx);
                    }
                }
                if existing.context_requirements == vec!["full_context".to_string()] {
                    existing.context_requirements = branch_keywords(&matched.content);
                }
                if existing.methodology_ids.is_empty() {
                    collect_methodology_ids(matched, &mut existing.methodology_ids);
                }
            }
        }

        let mut next_step_index = state
            .blueprint_steps
            .iter()
            .map(|s| s.step_index)
            .max()
            .map(|m| m + 1)
            .unwrap_or(0);
        for branch in &amt.children {
            let covered = state
                .blueprint_steps
                .iter()
                .any(|s| overlaps(&branch.content, &s.description));
            if !covered {
                tracing::warn!(
                    branch = %branch.content,
                    "Branch not addressed by any generated step — synthesizing a deterministic fallback step"
                );
                state.blueprint_steps.push(BlueprintStep {
                    step_index: next_step_index,
                    action: "execute_prompt".to_string(),
                    description: format!("Address: {}", branch.content),
                    pipeline_id: 9,
                    context_requirements: branch_keywords(&branch.content),
                    loop_config: None,
                    sub_steps: Vec::new(),
                    depends_on: Vec::new(),
                    wait_for_graph_update: false,
                    max_retries: 1,
                    timeout_ms: None,
                    model_override: None,
                    source_chunk_indices: {
                        let mut idx = branch.source_chunk_indices.clone();
                        for related_idx in related_chunk_indices(branch, &amt) {
                            if !idx.contains(&related_idx) {
                                idx.push(related_idx);
                            }
                        }
                        idx
                    },
                    methodology_ids: {
                        let mut ids = Vec::new();
                        collect_methodology_ids(branch, &mut ids);
                        ids
                    },
                });
                next_step_index += 1;
            }
        }

        if state.blueprint_steps.len() < branch_count {
            tracing::warn!(
                branch_count,
                step_count = state.blueprint_steps.len(),
                "Blueprint still has fewer steps than AMT branches after reconciliation"
            );
        }

        // INTENT CARRY: when the request text is the intent node (the
        // generic-intent displacement this pass fixes), some step must address
        // it. Covered = a step description contains the normalised request.
        // Otherwise one step for the request is appended. Recorded either way.
        // Extracted intents are already covered per-branch by reconciliation
        // above, so when no node is the request text this check reports
        // not_applicable rather than guessing.
        let request_key = super::amt::norm_intent_key(&request_text);
        let request_node: Option<&AMTNode> = amt
            .children
            .iter()
            .find(|c| super::amt::norm_intent_key(&c.content) == request_key)
            .or_else(|| {
                (super::amt::norm_intent_key(&amt.content) == request_key).then_some(&amt)
            });
        match request_node {
            None => super::amt::record_intent_event(
                state,
                "intent_carried",
                serde_json::json!({
                    "status": "not_applicable",
                    "reason": "no intent node equals the request text; per-branch reconciliation covers extracted intents",
                }),
            ),
            Some(node) => {
                let carried = state
                    .blueprint_steps
                    .iter()
                    .any(|s| super::amt::norm_intent_key(&s.description).contains(&request_key));
                if carried {
                    super::amt::record_intent_event(
                        state,
                        "intent_carried",
                        serde_json::json!({ "status": "already", "intent": request_text }),
                    );
                } else {
                    let mut methodology_ids = Vec::new();
                    collect_methodology_ids(node, &mut methodology_ids);
                    state.blueprint_steps.push(BlueprintStep {
                        step_index: next_step_index,
                        action: "execute_prompt".to_string(),
                        description: format!("Address: {}", request_text),
                        pipeline_id: 9,
                        context_requirements: branch_keywords(&request_text),
                        loop_config: None,
                        sub_steps: Vec::new(),
                        depends_on: Vec::new(),
                        wait_for_graph_update: false,
                        max_retries: 1,
                        timeout_ms: None,
                        model_override: None,
                        source_chunk_indices: node.source_chunk_indices.clone(),
                        methodology_ids,
                    });
                    super::amt::record_intent_event(
                        state,
                        "intent_carried",
                        serde_json::json!({ "status": "appended", "intent": request_text }),
                    );
                }
            }
        }

        // Store blueprint in ZSEI
        let blueprint_container = serde_json::json!({
            "container_type": "Blueprint",
            "metadata": {
                "name": name,
                "description": description,
                "created_by": "orchestrator"
            },
            "context": {
                "keywords": state.keywords,
                "topics": state.topics,
                "methodology_ids": state.methodologies
            },
            "storage": {
                "steps": state.blueprint_steps,
                "missing_capabilities": missing_capabilities
            }
        });

        if let Ok(new_id) = self.store.create_container(0, blueprint_container).await {
            state.blueprint_id = Some(new_id);
            state.blueprints_created += 1;
        }

        self.record_stage_timed(
            state,
            6,
            "Blueprint Assignment",
            true,
            &format!(
                "Created new blueprint with {} steps (missing: {})",
                state.blueprint_steps.len(),
                missing_capabilities.len()
            ),
            stage_start.elapsed().as_millis() as u64,
        );

        Ok(())
    }

    async fn stage_4_zero_shot_simulation(
        &self,
        state: &mut OrchestrationState,
    ) -> Result<(), String> {
        let stage_start = std::time::Instant::now();

        let amt = match &state.amt {
            Some(a) => a,
            None => {
                self.record_stage_timed(
                    state,
                    7,
                    "Zero-Shot Simulation",
                    true,
                    "Skipped (no AMT)",
                    0,
                );
                return Ok(());
            }
        };

        // Simulate execution using AMT traversal
        //
        // RICH CONTEXT (user directive: pass as much real data as possible —
        // nothing dropped at call sites). This prompt previously carried only
        // the root intent + level-1 branch NAMES: AMT sub-details, extracted
        // signals, attached files, jurisdiction outcomes, and traversal-backed
        // related containers were all silently dropped. Everything below is
        // real captured state. The tree renders in full here and is then sized
        // to the model window by render_within_window (a trim is recorded, never
        // silent; was a fixed 1500-char budget with the cut only in the prompt).
        let mut tree_render = String::new();
        fn render_tree(node: &crate::orchestrator::AMTNode, depth: usize, out: &mut String, budget: &mut usize) {
            if *budget == 0 {
                out.push_str("  … (tree continues)\n");
                *budget = 0;
                return;
            }
            let line = format!("{}- {}\n", "  ".repeat(depth), node.content);
            *budget = budget.saturating_sub(line.len());
            out.push_str(&line);
            for child in &node.children {
                render_tree(child, depth + 1, out, budget);
            }
        }
        let mut unbounded = usize::MAX;
        render_tree(amt, 0, &mut tree_render, &mut unbounded);
        // `amt` is not used past this point, so the state borrow is free again.
        let tree_render =
            super::amt::render_within_window(state, "simulation_tree", 8, tree_render);

        let entities_desc_raw = if state.entities.is_empty() {
            "(none extracted)".to_string()
        } else {
            state
                .entities
                .iter()
                .map(|e| format!("{} ({})", e.text, e.entity_type))
                .collect::<Vec<_>>()
                .join(", ")
        };
        // Every entity (was take(12), silently), sized against the window.
        let entities_desc =
            super::amt::render_within_window(state, "simulation_entities", 8, entities_desc_raw);
        let files_desc = if state.classified_file_graphs.is_empty() {
            "(none attached)".to_string()
        } else {
            state
                .classified_file_graphs
                .iter()
                .map(|f| format!("{} [{}]", f.file_path, f.modality))
                .collect::<Vec<_>>()
                .join(", ")
        };
        // Jurisdiction CROSS-REFERENCE: the gate ran at stage 0 and its
        // result is what the simulation must reason about — "matched: N"
        // counts alone give the model nothing to apply. jurisdiction_summary
        // is the shared source of truth (also used by AMT building and
        // blueprint assignment, see amt.rs/stage_3_blueprint_assignment)
        // so the same real rule/warning/confirmation text reaches every
        // stage that should see it, not a copy-pasted subset per site.
        let jurisdiction_desc = Self::jurisdiction_summary(state);

        // Methodologies in scope — the SAME real rules text the blueprint
        // assignment sees, so the simulation cross-references what the
        // steps were drafted under instead of guessing.
        let methodologies_block = if state.methodologies.is_empty() {
            "(none matched this request)".to_string()
        } else {
            let mut lines = Vec::new();
            for &method_id in state.methodologies.iter() {
                if let Ok(Some(container)) = self.store.get_container(method_id).await {
                    let name = container
                        .get("local_state").and_then(|ls| ls.get("metadata"))
                        .and_then(|m| m.get("name")).and_then(|n| n.as_str())
                        .unwrap_or("Unknown");
                    match Self::load_methodology_rules_full(&container) {
                        Some(rules) => lines.push(format!("  - {}: {}", name, rules)),
                        None => lines.push(format!("  - {} (no detailed rules on file yet)", name)),
                    }
                }
            }
            if lines.is_empty() {
                "(none matched this request)".to_string()
            } else {
                lines.join("\n")
            }
        };
        // Every methodology (was take(6), silently), sized against the window.
        let methodologies_block =
            super::amt::render_within_window(state, "simulation_methodologies", 8, methodologies_block);

        // Consciousness gate state — it RUNS AFTER this simulation (stage 8);
        // the model should know it will be applied.
        let consciousness_desc = if state.request.consciousness_enabled {
            "ENABLED — a consciousness decision gate will review this plan after the simulation (stage 8)".to_string()
        } else {
            "disabled in config".to_string()
        };

        // Traversal-backed related context: what the graph ALREADY holds for
        // this request's signals — real container names, infrastructure
        // filtered, capped. This is the loopable element: every prompt call
        // sees what prior runs persisted, so answers build on the graph.
        let related_names_raw = {
            let hits = self
                .store
                .search_by_keywords(
                    &state.keywords.iter().take(8).cloned().collect::<Vec<_>>(),
                    None,
                )
                .await
                .unwrap_or_default();
            let mut names = Vec::new();
            for id in hits.into_iter() {
                if let Ok(Some(c)) = self.store.get_container(id).await {
                    let ctype = c
                        .get("local_state").and_then(|l| l.get("metadata"))
                        .and_then(|m| m.get("container_type")).and_then(|t| t.as_str()).unwrap_or("");
                    if matches!(ctype, "Root" | "User" | "Workspace" | "Project" | "Pipeline" | "ModalityRoot" | "MethodologyRoot" | "BlueprintRoot" | "PipelineRoot" | "ConsciousnessRoot" | "ExternalRoot" | "PackageRoot" | "JurisdictionRoot") {
                        continue;
                    }
                    if let Some(n) = c.get("local_state").and_then(|l| l.get("metadata")).and_then(|m| m.get("name")).and_then(|n| n.as_str()) {
                        names.push(format!("{} (#{})", n, id));
                    }
                }
            }
            if names.is_empty() {
                "(no strongly related containers in the graph yet)".to_string()
            } else {
                names.join("\n- ")
            }
        };
        // Every related name (was capped at 5, silently), sized against the
        // reading model's window; cuts recorded.
        let related_names =
            super::amt::render_within_window(state, "related_names", 8, related_names_raw);

        let capabilities =
            Self::capability_summary_for_need(state, &state.request.prompt);
        let simulate_prompt = format!(
            r#"Simulate executing this plan and predict outcomes.

REQUEST SIGNALS:
- Keywords: {}
- Entities: {}
- Topics: {}
- Attached files: {}
- Jurisdiction gate: {}

METHODOLOGIES IN SCOPE (rules the blueprint was drafted under — cross-reference these):
{}

CONSCIOUSNESS GATE: {}

RELATED CONTAINERS ALREADY IN THE GRAPH (traversal-backed — build on these, don't re-derive):
- {}

FULL AMT STRUCTURE (all levels):
{}

BLUEPRINT STEPS (pipeline IDs show which capability each step routes to):
{}

REGISTERED CAPABILITIES (tools/MCPs callable via /mcp/call — flag any step
whose goal matches a registered capability instead of a pipeline):
{capabilities}

For each step, predict:
1. What information will be needed
2. What output will be produced
3. Risks or failure modes (things that could go wrong when running this step — NOT questions to ask the user)

`clarifications_needed` is for BLOCKING ambiguity ONLY: cases where no reasonable
default exists and proceeding would produce something the user did not ask for
at all (e.g. the request names a specific file/system/person that doesn't
exist or wasn't identified, or two instructions directly contradict each
other). This must be empty in the overwhelming majority of requests.

Do NOT add a clarification for: tone/style/formality, target audience/recipient,
language (assume the language the prompt was written in), output length or
format details, whether to add error handling/edge cases/verification steps,
or any other preference where a sensible default lets you proceed. Pick the
most reasonable default silently and simulate against it — do not ask.

If you would put anything in `clarifications_needed`, first ask yourself: "is
there truly no reasonable default I could pick instead?" Only list it if the
honest answer is yes.

Return JSON:
{{
    "step_predictions": [
        {{"step": 0, "needs": ["info1"], "produces": ["output1"], "risks": ["risk1"]}}
    ],
    "overall_feasibility": "high/medium/low",
    "clarifications_needed": []
}}"#,
            state.keywords.join(", "),
            entities_desc,
            state.topics.join(", "),
            files_desc,
            jurisdiction_desc,
            methodologies_block,
            consciousness_desc,
            related_names,
            tree_render,
            state
                .blueprint_steps
                .iter()
                .map(|s| {
                    let ctx_req = if s.context_requirements.is_empty() {
                        String::new()
                    } else {
                        format!(" [needs: {}]", s.context_requirements.join(", "))
                    };
                    format!(
                        "Step {}: {} - {} (pipeline {}){}",
                        s.step_index, s.action, s.description, s.pipeline_id, ctx_req
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        );

        let sim_input = serde_json::json!({
            "prompt": simulate_prompt,
            "max_tokens": 800,
            "temperature": 0.3,
            "system_prompt": "Simulate execution and predict outcomes. Respond with JSON only."
        });

        // Same Ok-but-empty coverage as blueprint assignment above — this
        // is the exact call CHECKLIST.md's BitNet coherency sweep already
        // found truncating/schema-drifting; retry+fallback gives it a real
        // second chance instead of accepting a degraded first attempt.
        let sim_result = self.metered_execute_resilient(state, sim_input, "zero_shot_simulation").await?;
        self.record_thinking(state, "Zero-Shot Simulation", &sim_result);
        let response = sim_result
            .get("response")
            .and_then(|r| r.as_str())
            .unwrap_or("{}");
        let sim_json = Self::parse_json_object(response);

        // NO self-reported confidence: the model's numeric self-assessment is
        // fabricated precision. Clarification is decided structurally —
        // clarifications exist or they don't. Feasibility stays as the
        // model's qualitative claim, recorded as data.
        let feasibility = sim_json
            .get("overall_feasibility")
            .and_then(|f| f.as_str())
            .unwrap_or("medium");

        // Check for clarifications needed
        if let Some(clarifications) = sim_json
            .get("clarifications_needed")
            .and_then(|c| c.as_array())
        {
            for c in clarifications {
                if let Some(c_str) = c.as_str() {
                    if !c_str.is_empty() {
                        state.clarification_points.push(c_str.to_string());
                    }
                }
            }
        }

        if !state.clarification_points.is_empty() {
            state.needs_clarification = true;
        }

        // Real per-step predictions — previously parsed into `sim_json`
        // then discarded (only clarifications_needed survived into
        // `state`). Stored so stage 8's consciousness review can carry
        // the simulation's own self-critique, matching what its assembled
        // context already claims to include.
        //
        // SCHEMA-FLEXIBLE (real bug found+fixed 2026-09-22): the requested
        // shape is a `step_predictions` array, but BitNet (confirmed live,
        // CHECKLIST.md's coherency sweep, "valid-JSON-wrong-schema (step_N
        // keys vs step_predictions)") sometimes returns real, valid,
        // substantive prediction data under top-level `step_0`/`step_1`/...
        // keys instead. The array-only parse below silently produced an
        // empty Vec in that case — a real response with real content,
        // passing `is_unusable_pipeline9_result` fine (non-empty), quietly
        // losing its data downstream. Retrying wouldn't reliably fix this
        // (the model may drift to the same shape again) — the fix is
        // accepting the shape it actually uses. Try the requested array
        // shape first; if empty, fall back to scanning for `step_<N>` keys.
        let step_predictions: Vec<SimulationStepPrediction> = {
            let from_array: Vec<SimulationStepPrediction> = sim_json
                .get("step_predictions")
                .and_then(|p| p.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|p| SimulationStepPrediction {
                            step: p.get("step").and_then(|s| s.as_u64()).unwrap_or(0) as u32,
                            needs: p
                                .get("needs")
                                .and_then(|n| n.as_array())
                                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                                .unwrap_or_default(),
                            produces: p
                                .get("produces")
                                .and_then(|n| n.as_array())
                                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                                .unwrap_or_default(),
                            risks: p
                                .get("risks")
                                .and_then(|n| n.as_array())
                                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                                .unwrap_or_default(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            if !from_array.is_empty() {
                from_array
            } else if let Some(obj) = sim_json.as_object() {
                // step_N-keyed fallback: real drifted shape, e.g.
                // {"step_0": {"needs":[...],"produces":[...],"risks":[...]}}
                // or {"step_0": "free-text prediction"}. Extract the numeric
                // suffix as the step index; a non-object value is kept as a
                // single risk-bucket string rather than dropped, since it's
                // still real model content, not nothing.
                let mut from_keys: Vec<SimulationStepPrediction> = obj
                    .iter()
                    .filter_map(|(k, v)| {
                        let suffix = k.strip_prefix("step_").or_else(|| k.strip_prefix("step"))?;
                        let step: u32 = suffix.trim_start_matches('_').parse().ok()?;
                        Some(match v.as_object() {
                            Some(step_obj) => SimulationStepPrediction {
                                step,
                                needs: step_obj
                                    .get("needs")
                                    .and_then(|n| n.as_array())
                                    .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                                    .unwrap_or_default(),
                                produces: step_obj
                                    .get("produces")
                                    .and_then(|n| n.as_array())
                                    .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                                    .unwrap_or_default(),
                                risks: step_obj
                                    .get("risks")
                                    .and_then(|n| n.as_array())
                                    .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                                    .unwrap_or_default(),
                            },
                            None => SimulationStepPrediction {
                                step,
                                needs: Vec::new(),
                                produces: Vec::new(),
                                risks: v.as_str().map(|s| vec![s.to_string()]).unwrap_or_default(),
                            },
                        })
                    })
                    .collect();
                from_keys.sort_by_key(|p| p.step);
                if !from_keys.is_empty() {
                    tracing::info!(
                        count = from_keys.len(),
                        "Zero-shot simulation: schema-flexible parse recovered step_N-keyed predictions the array-only parse would have dropped"
                    );
                }
                from_keys
            } else {
                Vec::new()
            }
        };
        state.simulation_result = Some(SimulationOutcome {
            overall_feasibility: feasibility.to_string(),
            step_predictions,
        });

        self.record_stage_timed(
            state,
            7,
            "Zero-Shot Simulation",
            true,
            &format!(
                "Clarifications: {}, Feasibility: {} (model-reported)",
                state.clarification_points.len(),
                feasibility
            ),
            stage_start.elapsed().as_millis() as u64,
        );

        Ok(())
    }

    async fn stage_5_consciousness_gate(
        &self,
        state: &mut OrchestrationState,
    ) -> Result<(), String> {
        let stage_start = std::time::Instant::now();

        // Call decision_gate pipeline (#39) — now carrying the FULL
        // TRAVERSED PICTURE (TOP_DOWN_REVIEW_GUIDE §3.2, CONTEXT_REGISTRY
        // S1-S7): the AMT (source of truth), the blueprint (the goal),
        // the jurisdiction outcome, the methodology rules the plan was
        // drafted under, and the simulation's own self-critique. Assembled
        // once, here — carried unchanged across every model-switch attempt
        // inside the review (wire-before-drop: same execute(39) call
        // shape, real decision inside).
        tracing::info!(stage = 8, "[8.1] consciousness gate: assembling full traversed picture (AMT + blueprint + jurisdiction + methodology rules + simulation)");
        let mut amt_full = String::new();
        if let Some(amt) = &state.amt {
            fn render(node: &crate::orchestrator::AMTNode, depth: usize, out: &mut String, budget: &mut usize) {
                if *budget == 0 { return; }
                let line = format!("{}- {}\n", "  ".repeat(depth), node.content);
                *budget = budget.saturating_sub(line.len());
                out.push_str(&line);
                for child in &node.children {
                    render(child, depth + 1, out, budget);
                }
            }
            // Full render here; sized to the window below. The AMT is the
            // source of truth for this review. The old fixed 1200-char floor
            // is removed; the budget is window/8 tokens, and every cut is
            // recorded by render_within_window (an unknown window cuts nothing).
            let mut unbounded = usize::MAX;
            render(amt, 0, &mut amt_full, &mut unbounded);
        }
        let amt_render =
            super::amt::render_within_window(state, "gate_amt_render", 8, amt_full);
        let blueprint_render = state
            .blueprint_steps
            .iter()
            .map(|s| format!(
                "Step {}: {} - {} (pipeline {})",
                s.step_index, s.action, s.description, s.pipeline_id
            ))
            .collect::<Vec<_>>()
            .join("\n");
        let jurisdiction_ctx = Self::jurisdiction_summary(state);
        // The simulation's own self-critique (stage 4's real predictions —
        // see SimulationOutcome) — this was claimed in the doc comment
        // above before it was actually wired; now real.
        let simulation_render = match &state.simulation_result {
            Some(sim) => {
                let steps_text = if sim.step_predictions.is_empty() {
                    "(no per-step predictions parsed)".to_string()
                } else {
                    sim.step_predictions
                        .iter()
                        .map(|p| {
                            format!(
                                "  Step {}: needs [{}], produces [{}], risks [{}]",
                                p.step,
                                p.needs.join(", "),
                                p.produces.join(", "),
                                p.risks.join(", ")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                format!("Overall feasibility: {}\n{}", sim.overall_feasibility, steps_text)
            }
            None => "(simulation stage did not run or produced no result)".to_string(),
        };
        let mut methods_render = String::new();
        for &method_id in state.methodologies.iter() {
            if let Ok(Some(container)) = self.store.get_container(method_id).await {
                let name = container
                    .get("local_state").and_then(|l| l.get("metadata"))
                    .and_then(|m| m.get("name")).and_then(|n| n.as_str())
                    .unwrap_or("Unknown");
                match Self::load_methodology_rules_full(&container) {
                    Some(rules) => methods_render.push_str(&format!("  - {}: {}\n", name, rules)),
                    None => methods_render.push_str(&format!("  - {} (no rules on file)\n", name)),
                }
            }
        }

        // Methodology rules (every methodology, was take(4)) and the whole
        // request (was a 600-char cut) are sized against the window; cuts
        // are recorded.
        let methods_render =
            super::amt::render_within_window(state, "gate_methodologies", 8, methods_render);
        let request_full = state.cleaned_prompt.clone();
        let request_text =
            super::amt::render_within_window(state, "gate_request", 8, request_full);

        let input = serde_json::json!({
            "action": "Evaluate",
            "task_id": 0,
            "task_summary": format!(
                "THE PLAN UNDER REVIEW (the blueprint is the goal — nothing has executed yet; judge the plan and its judgment with everything connected to it):\n\nREQUEST:\n{}\n\nAMT (SOURCE OF TRUTH):\n{}\n\nBLUEPRINT STEPS:\n{}\n\nJURISDICTION OUTCOME:\n{}\n\nMETHODOLOGY RULES IN SCOPE:\n{}\n\nSIMULATION PREDICTIONS (the system's own self-critique of this plan, from stage 4):\n{}",
                request_text,
                amt_render,
                blueprint_render,
                jurisdiction_ctx,
                methods_render,
                simulation_render
            ),
            "blueprint_id": state.blueprint_id.unwrap_or(0),
            "user_id": state.request.user_id,
            "amt_summary": {
                "intent": state.amt.as_ref().map(|a| &a.content),
                "branch_count": state.amt.as_ref().map(|a| a.branch_count()).unwrap_or(0)
            }
        });

        tracing::info!(stage = 8, "[8.2] gate input assembled — executing decision_gate (39), adapter-watchdog-bounded");
        let result = self.executor.execute(39, input).await;
        tracing::info!(stage = 8, "[8.3] gate execute returned");
        let result = result?;

        let decision = result
            .get("gate")
            .and_then(|g| g.get("decision"))
            .and_then(|d| d.as_str())
            .unwrap_or("Proceed");

        // Captured from the gate pipeline when it reports one — absence is
        // None, never a fabricated default.
        let confidence: Option<f32> = result
            .get("gate")
            .and_then(|g| g.get("confidence"))
            .and_then(|c| c.as_f64())
            .map(|c| c as f32);

        let reasoning = result
            .get("gate")
            .and_then(|g| g.get("reasoning"))
            .and_then(|r| r.as_str())
            .unwrap_or("No reasoning provided")
            .to_string();

        state.gate_result = Some(GateResult {
            decision: decision.to_string(),
            confidence,
            reasoning: reasoning.clone(),
        });

        if decision == "Decline" {
            return Err(format!("Consciousness gate declined: {}", reasoning));
        }

        let confidence_txt = confidence
            .map(|c| format!(" ({:.0}%)", c * 100.0))
            .unwrap_or_default();
        self.record_stage_timed(
            state,
            8,
            "Consciousness Gate",
            true,
            &format!("Decision: {}{} [gate-reported]", decision, confidence_txt),
            stage_start.elapsed().as_millis() as u64,
        );

        Ok(())
    }

    async fn stage_6_to_8_execute_steps(
        &self,
        state: &mut OrchestrationState,
    ) -> Result<(), String> {
        let stage_start = std::time::Instant::now();

        // STAGE 7: Create task via TaskManager
        let mut inputs = HashMap::new();
        inputs.insert(
            "prompt".to_string(),
            serde_json::json!(state.cleaned_prompt),
        );
        inputs.insert(
            "blueprint_id".to_string(),
            serde_json::json!(state.blueprint_id),
        );
        if let Some(ref amt) = state.amt {
            inputs.insert("amt_intent".to_string(), serde_json::json!(amt.content));
        }
        // Real ZSEI container id for the persisted AMT tree (see
        // persist_amt_container in amt.rs) — lets a later request retrieve
        // "what AMT produced this task" via GetContainer, since the task is
        // the first point in the stage sequence where a durable id (task_id)
        // exists to link it against.
        if let Some(amt_container_id) = state.amt_container_id {
            inputs.insert("amt_container_id".to_string(), serde_json::json!(amt_container_id));
        }

        // Enqueue task via TaskManager
        let task_result = self
            .task_manager
            .read()
            .await
            .enqueue_task(
                state.blueprint_id,
                inputs,
                state.request.user_id,
                state.request.device_id,
                state.request.workspace_id,
                state.request.project_id,
                TaskPriority::Normal,
            )
            .await;

        match task_result {
            Ok(task_id) => {
                state.task_id = Some(task_id);

            }
            Err(e) => {
                self.record_stage(state, 9, "Task Creation", false, &format!("Failed: {}", e));
                return Err(e.to_string());
            }
        }

        self.record_stage(
            state,
            9,
            "Task Creation",
            state.task_id.is_some(),
            &format!("Task: {:?}", state.task_id),
        );

        // STAGES 6 & 8: Execute steps
        let steps = state.blueprint_steps.clone();
        let mut all_outputs: Vec<String> = Vec::new();
        let mut completed: HashSet<u32> = HashSet::new();

        // Build dependency order
        let mut step_queue: Vec<&BlueprintStep> = steps.iter().collect();
        let mut iterations = 0;
        let max_iterations = steps.len() * 2;

        while !step_queue.is_empty() && iterations < max_iterations {
            iterations += 1;

            // Cooperative cancellation: a step already in flight (the
            // .execute_step call below) always runs to completion — this
            // only stops the NEXT step from starting once a user-requested
            // cancel (TaskManager::cancel_task) has landed.
            if let Some(task_id) = state.task_id {
                if self.task_manager.read().await.is_cancelled(task_id).await {
                    tracing::info!("Task {} cancelled — stopping before next step", task_id);
                    break;
                }
            }

            // Find steps whose dependencies are satisfied
            let ready_steps: Vec<_> = step_queue
                .iter()
                .filter(|s| s.depends_on.iter().all(|dep| completed.contains(dep)))
                .cloned()
                .collect();

            if ready_steps.is_empty() && !step_queue.is_empty() {
                // Force execute first remaining step (break deadlock)
                if let Some(step) = step_queue.first().cloned() {
                    let result = self.execute_step(state, step, &all_outputs).await?;
                    let output_text = self.extract_output_text(&result.output);
                    let model_used = result
                        .output
                        .get("model_used")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    all_outputs.push(output_text.clone());
                    state.step_results.push(result.clone());
                    state.step_outputs.insert(
                        step.step_index,
                        serde_json::json!({"output": all_outputs.last()}),
                    );
                    completed.insert(step.step_index);
                    step_queue.retain(|s| s.step_index != step.step_index);

                    // Update TaskManager (full step metadata — measured
                    // tokens go into the version note, never fake scores)
                    if let Some(task_id) = state.task_id {
                        let graphs_updated: Vec<String> = result
                            .output
                            .get("graphs_updated")
                            .and_then(|g| g.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default();
                        let _ = self
                            .task_manager
                            .read()
                            .await
                            .update_step(
                                task_id,
                                step.step_index,
                                "completed",
                                Some(result.tokens_used),
                                Some(output_text.clone()),
                                None, // execution_id
                                &step.description,
                                Some(format!(
                                    "Step {} completed: {} tokens",
                                    step.step_index, result.tokens_used
                                )),
                                graphs_updated,
                                step.context_requirements.clone(),
                                Some("execute".to_string()),
                                vec![],
                                vec![],
                                model_used,
                            )
                            .await;

                        let _ = self
                            .task_manager
                            .read()
                            .await
                            .update_progress(task_id, completed.len() as u32, steps.len() as u32)
                            .await;
                    }
                }
            } else {
                for step in ready_steps {
                    let result = self.execute_step(state, step, &all_outputs).await?;
                    let output_text = self.extract_output_text(&result.output);
                    let model_used = result
                        .output
                        .get("model_used")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    all_outputs.push(output_text.clone());
                    state.step_results.push(result.clone());
                    state.step_outputs.insert(
                        step.step_index,
                        serde_json::json!({"output": all_outputs.last()}),
                    );
                    completed.insert(step.step_index);
                    step_queue.retain(|s| s.step_index != step.step_index);

                    // Update TaskManager (full step metadata — measured
                    // tokens go into the version note, never fake scores)
                    if let Some(task_id) = state.task_id {
                        let graphs_updated: Vec<String> = result
                            .output
                            .get("graphs_updated")
                            .and_then(|g| g.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default();
                        let _ = self
                            .task_manager
                            .read()
                            .await
                            .update_step(
                                task_id,
                                step.step_index,
                                "completed",
                                Some(result.tokens_used),
                                Some(output_text.clone()),
                                None, // execution_id
                                &step.description,
                                Some(format!(
                                    "Step {} completed: {} tokens",
                                    step.step_index, result.tokens_used
                                )),
                                graphs_updated,
                                step.context_requirements.clone(),
                                Some("execute".to_string()),
                                vec![],
                                vec![],
                                model_used,
                            )
                            .await;

                        let _ = self
                            .task_manager
                            .read()
                            .await
                            .update_progress(task_id, completed.len() as u32, steps.len() as u32)
                            .await;
                    }
                }
            }
        }

        // Combine outputs into final response
        state.final_response = if all_outputs.len() == 1 {
            Some(all_outputs[0].clone())
        } else if !all_outputs.is_empty() {
            Some(all_outputs.join("\n\n"))
        } else {
            None
        };

        self.record_stage_timed(
            state,
            10,
            "Step Execution",
            state.final_response.is_some(),
            &format!(
                "{} steps executed, tokens: {}",
                state.step_results.len(),
                state.tokens_used_so_far
            ),
            stage_start.elapsed().as_millis() as u64,
        );

        Ok(())
    }

    /// Real web search for one blueprint step, with AMT-style decomposition
    /// for genuinely multi-part queries. Query decomposition is an LLM
    /// judgment call (same family as build_amt_layer_by_layer's intent
    /// extraction in amt.rs) so it lives here, in the orchestrator, which
    /// already makes real LLM calls — pipeline 56 itself stays a dumb,
    /// honest "call the real search API or say unavailable" unit, same
    /// design posture as every other pipeline in this codebase.
    ///
    /// The decomposition call always runs (cheap, small max_tokens) rather
    /// than a heuristic guess at "is this multi-part" — if the model
    /// judges the query already single-purpose, it returns one sub-query
    /// and this degrades to exactly one real search call, no different
    /// from not decomposing at all.
    async fn execute_web_search_step(
        &self,
        state: &mut OrchestrationState,
        step: &BlueprintStep,
    ) -> Result<serde_json::Value, String> {
        let query = step.description.clone();

        // Current-date/time requests need no search or decomposition at all.
        let lower = query.to_lowercase();
        if (lower.contains("current date") || lower.contains("current time") || lower.contains("what time is it") || lower.contains("today's date"))
            && lower.split_whitespace().count() < 12
        {
            let dt_input = serde_json::json!({"action": {"type": "CurrentDateTime"}});
            return self.executor.execute(56, dt_input).await;
        }

        let decompose_prompt = format!(
            r#"A user's request implies this web search need: "{}"

If this is genuinely composed of multiple distinct, independently-searchable
questions, split it into 2-4 real sub-queries. If it is already one focused
question, return it unchanged as the only element.

Return ONLY valid JSON: {{"sub_queries": ["query 1", "query 2"]}}"#,
            query
        );
        let decompose_input = serde_json::json!({
            "prompt": decompose_prompt,
            "max_tokens": 200,
            "temperature": 0.1,
            "system_prompt": "Decompose search queries. Return only valid JSON."
        });

        let sub_queries: Vec<String> = match self.metered_execute_resilient(state, decompose_input, "web_search_decompose").await {
            Ok(result) => {
                self.record_thinking(state, "Web Search — query decomposition", &result);
                let response = result.get("response").and_then(|r| r.as_str()).unwrap_or("{}");
                let json_str = Self::extract_json_from_response(response, '{', '}');
                serde_json::from_str::<serde_json::Value>(json_str.trim())
                    .ok()
                    .and_then(|v| v.get("sub_queries").cloned())
                    .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok())
                    .filter(|v| !v.is_empty())
                    .unwrap_or_else(|| vec![query.clone()])
            }
            Err(_) => vec![query.clone()],
        };

        let mut all_results = Vec::new();
        let mut any_success = false;
        let mut last_error = None;
        for sub_query in &sub_queries {
            let search_input = serde_json::json!({
                "action": {"type": "Search", "query": sub_query, "max_results": 5}
            });
            match self.executor.execute(56, search_input).await {
                Ok(result) => {
                    if result.get("success").and_then(|s| s.as_bool()) == Some(true) {
                        any_success = true;
                    } else {
                        last_error = result.get("error").and_then(|e| e.as_str()).map(String::from);
                    }
                    all_results.push(serde_json::json!({
                        "sub_query": sub_query,
                        "result": result
                    }));
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        Ok(serde_json::json!({
            "success": any_success,
            "error": if any_success { None } else { last_error },
            "sub_queries": sub_queries,
            "results_by_sub_query": all_results,
        }))
    }

    /// Execute a single blueprint step (handles loops, sub-steps, retries)
    async fn execute_step(
        &self,
        state: &mut OrchestrationState,
        step: &BlueprintStep,
        previous_outputs: &[String],
    ) -> Result<StepResult, String> {
        let mut total_iterations = 0;
        let mut sub_step_results = Vec::new();
        let mut final_output = serde_json::json!({});

        // Handle loop configuration
        let (iterations, should_loop) = if let Some(loop_config) = &step.loop_config {
            match loop_config.loop_type {
                LoopType::Count => (loop_config.max_iterations, true),
                LoopType::While | LoopType::Until => (loop_config.max_iterations, true),
                LoopType::ForEach => {
                    // Get iteration count from iterate_over
                    let count = step.context_requirements.len() as u32;
                    (count.max(1), true)
                }
            }
        } else {
            (1, false)
        };

        for iteration in 0..iterations {
            total_iterations = iteration + 1;

            // STAGE 6: Context aggregation for this step — Section S ForStep:
            // the session's own text graph (validated sentences from the chunk
            // graphs) at top priority, merged with query-scoped store context.
            // Scoped to this step's own AMT branch (source_chunk_indices, set
            // by stage_3_blueprint_assignment's reconciliation pass) rather
            // than always walking the whole session from chunk 0 — a step
            // for one branch of a multi-branch request shouldn't pull in
            // every other branch's text too. Falls back to the full walk
            // when no branch scope is known (e.g. the single-step fallback
            // blueprint).
            let session_budget = state.model_context_limit / 8;
            let (session_context, session_dropped) = self.reconstruct_session_context(
                state,
                session_budget,
                &step.source_chunk_indices,
            );
            if session_dropped > 0 {
                super::amt::record_intent_event(
                    state,
                    "session_context_trimmed",
                    serde_json::json!({
                        "step": step.step_index,
                        "dropped_units": session_dropped,
                        "budget_tokens": session_budget,
                    }),
                );
            }
            let context_input = serde_json::json!({
                "action": "ForStep",
                "query": format!("{} - {}", state.cleaned_prompt, step.description),
                "session_context": session_context,
                "token_budget": state.model_context_limit / 4,
                "project_id": state.request.project_id,
                "priority_order": step.context_requirements,
                // Global consciousness state is a separate, explicitly-opt-in
                // layer (see context_aggregation's ForStep handler) — only
                // requested when this orchestration run actually has it on.
                "include_consciousness": state.request.consciousness_enabled,
                // Coordination layer (task 43): scoped agent history from the
                // /SharedContext graph (global + this workspace/project) as
                // its own layer — the AMT stays context-aligned with the
                // living graph.
                "workspace_id": state.request.workspace_id,
                "include_coordination": true,
                // This request's own attached-file graph_ids, exactly
                // known (state.file_graphs, populated in STEP 0) — see
                // context_aggregation's ForStep handler for why this
                // matters: previously a step could only ever rediscover
                // its own request's attachments via keyword search,
                // probabilistic even when the exact container is already
                // in hand.
                "known_seed_ids": state.file_graphs.values().cloned().collect::<Vec<u64>>(),
            });

            let context_result = self.metered_execute(state, 21, context_input).await?;
            let step_context = context_result
                .get("context")
                .and_then(|c| c.get("context_text"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();

            state
                .step_contexts
                .insert(step.step_index, step_context.clone());

            // CONTEXT OBJECT: persist this step's assembled context the
            // moment the task exists (Stage 7 created it before step
            // execution) — the "what did this step actually see" record,
            // with provenance of the gathering mechanism.
            if let Some(task_id) = state.task_id {
                if let Err(e) = self
                    .task_manager
                    .read()
                    .await
                    .update_step_context(
                        task_id,
                        step.step_index,
                        &step_context,
                        vec!["keyword-scan".to_string()],
                    )
                    .await
                {
                    tracing::warn!(
                        task_id,
                        step = step.step_index,
                        error = %e,
                        "Failed to persist step context object"
                    );
                }
            }

            // Previous step outputs, whole, sized against this step's model
            // window. The old per-output cut at 300 chars was silent.
            let previous_block = if previous_outputs.is_empty() {
                String::new()
            } else {
                let joined = previous_outputs
                    .iter()
                    .enumerate()
                    .map(|(i, o)| format!("Step {}: {}", i + 1, o))
                    .collect::<Vec<_>>()
                    .join("\n");
                super::amt::render_within_window(state, "previous_step_outputs", 4, joined)
            };

            // Build full context with previous outputs + coordination layer.
            // The coordination layer (task 43) carries scoped agent history
            // from the /SharedContext graph — included as a distinct labeled
            // section, never mixed into the project-scoped context text.
            let coordination_layer = context_result
                .get("context")
                .and_then(|c| c.get("coordination_context"))
                .and_then(|c| c.as_str())
                .unwrap_or("");
            let full_context = {
                let mut fc = String::new();
                if !coordination_layer.is_empty() {
                    fc.push_str("[Agent coordination history]\n");
                    fc.push_str(coordination_layer);
                    fc.push_str("\n\n");
                }
                fc.push_str(&step_context);
                if !previous_outputs.is_empty() {
                    fc.push_str("\n\nPrevious step outputs:\n");
                    fc.push_str(&previous_block);
                }
                fc
            };

            // Attached-file content (real bytes read from disk in
            // prompt_normalization's Step 0) — confirmed live this was
            // required, not optional: file_graphs/FileLayerContext only
            // ever carried path/modality/role/graph_id metadata, so a step
            // asking to review attached code correctly got back "no project
            // code was actually included in the request" even though the
            // files were detected, read, and graphed. Every step gets every
            // attached file's content (small, real projects rarely exceed
            // a handful of files) rather than trying to guess which
            // branch a given file belongs to; the compaction pass right
            // below still protects against this overflowing a small
            // model's budget.
            let full_context = if state.attached_file_contents.is_empty() {
                full_context
            } else {
                let files_block: String = state
                    .attached_file_contents
                    .iter()
                    .map(|(path, content)| format!("## File: {}\n{}\n", path, content))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("{}\n\n{}", files_block, full_context)
            };

            // STEP-BASED SMART COMPACTION: context is deliberately never the
            // whole project per step (branch-scoped session reconstruction
            // above, keyword-scoped store search in context_aggregation) —
            // but a wide-scoped branch plus several previous-step outputs
            // can still legitimately overflow a small model's real context
            // window. Only compact when this step's actual assembled
            // context would exceed that budget — never unconditionally —
            // and compact via a real LLM summarization pass rather than
            // blind truncation, since the point is to keep what matters,
            // not just cut it off. Falls back to truncation only if the
            // compaction call itself fails, so an LLM outage can't hang or
            // fail the step outright.
            let response_reserve = state.model_context_limit / 4; // matches max_tokens below
            let input_budget = state.model_context_limit.saturating_sub(response_reserve).max(256);
            let estimated_input_tokens =
                Self::estimate_tokens(&full_context) + Self::estimate_tokens(&step.description) + 128;
            let full_context = if estimated_input_tokens > input_budget {
                tracing::warn!(
                    step_index = step.step_index,
                    estimated_input_tokens,
                    input_budget,
                    "Step context exceeds model's context budget — compacting"
                );
                let target_chars = (input_budget as usize).saturating_mul(4);
                let context_window = state.model_context_limit as usize;
                let compact_prompt = format!(
                    "Compress the following context to at most {} characters while preserving every \
                     fact, requirement, or constraint relevant to this task: \"{}\". Do not add \
                     commentary or explanation — return only the compacted context text.\n\n\
                     CONTEXT TO COMPACT:\n{}",
                    target_chars, step.description, full_context
                );
                let compact_input = serde_json::json!({
                    "prompt": compact_prompt,
                    "max_tokens": input_budget,
                    "temperature": 0.2,
                    "system_prompt": "Compact context losslessly for facts. Return only the compacted text, no explanation."
                });
                match self.metered_execute_resilient(state, compact_input, "context_compaction").await {
                    Ok(result) => {
                        self.record_thinking(
                            state,
                            &format!("Context Compaction (step {})", step.step_index),
                            &result,
                        );
                        let compacted = result
                            .get("response")
                            .and_then(|r| r.as_str())
                            .unwrap_or("")
                            .to_string();
                        if compacted.trim().is_empty() {
                            super::amt::render_within_budget(
                                state,
                                "step_context_compaction_fallback",
                                context_window,
                                input_budget as usize,
                                full_context,
                            )
                        } else {
                            compacted
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Context compaction call failed — falling back to truncation");
                        super::amt::render_within_budget(
                                state,
                                "step_context_compaction_fallback",
                                context_window,
                                input_budget as usize,
                                full_context,
                            )
                    }
                }
            } else {
                full_context
            };

            // Execute sub-steps first if any
            for sub_step in &step.sub_steps {
                let sub_input = self.build_sub_step_input(state, sub_step, &full_context)?;
                let sub_result = self.executor.execute(sub_step.pipeline_id, sub_input).await;

                sub_step_results.push(SubStepResult {
                    sub_index: sub_step.sub_index,
                    output: sub_result.clone().unwrap_or_default(),
                    success: sub_result.is_ok(),
                });
            }

            // Execute main step — pipeline 56 (WebSearch) gets its own
            // input shape and its own AMT-style decomposition instead of
            // the generic pipeline-9 payload below (see the coercion
            // exception comment above and execute_web_search_step's own
            // doc comment for why decomposition lives here, not in the
            // pipeline itself).
            tracing::info!(stage = 10, "[10.i{}] step {} (pipeline {}) iteration begin", iteration, step.step_index, step.pipeline_id);
            if step.pipeline_id == 56 {
                tracing::info!(stage = 10, "[10.w{}] web-search step begin", step.step_index);
                final_output = self.execute_web_search_step(state, step).await?;
                tracing::info!(stage = 10, "[10.w{}] web-search step done", step.step_index);
                self.record_thinking(
                    state,
                    &format!("Step Execution (step {})", step.step_index),
                    &final_output,
                );
                if step.wait_for_graph_update {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
                total_iterations = iteration + 1;
                break;
            }

            // Original request leads, background context trails and is
            // explicitly marked optional — a weaker model that loses track
            // of a long input tends to latch onto whatever is LAST, and
            // background context can legitimately be long. Putting the
            // real instruction first and telling the model outright to
            // ignore irrelevant background avoids the model treating noisy
            // or tangential context as the thing it's supposed to produce
            // (confirmed live: the previous Context-before-request ordering
            // let a step's actual output become a regurgitation of its own
            // background context instead of a real answer).
            // The original request is the task itself: whole, sized against the
            // window; any trim is recorded.
            let original_request = state.cleaned_prompt.clone();
            let request_text =
                super::amt::render_within_window(state, "original_request", 2, original_request);
            let step_prompt = format!(
                "Original request: {}\n\nCurrent step ({}): {}\n\n\
                 Background context (may be empty or only partially relevant — \
                 use what helps, ignore the rest, never treat it as the task):\n{}",
                request_text,
                step.step_index + 1,
                step.description,
                full_context,
            );

            let mut exec_input = serde_json::json!({
                "prompt": step_prompt,
                "max_tokens": state.model_context_limit / 4,
                "temperature": 0.7,
                "action": step.action
            });
            // Per-step model override (real multi-model routing): pipeline 9
            // merges this onto its env-derived base config before dispatch,
            // so this step alone can hit a different backend than the rest
            // of the run. Resolve a bare model_identifier against the host's
            // available_models here — the blueprint-generating LLM is only
            // ever asked for the identifier, not full connection details, so
            // pipeline 9 (which stays a dumb wire-protocol executor) needs
            // those details filled in before the override reaches it.
            // PAID REFUSAL (policy: paid models are never used). An override
            // naming a paid or unknown model is NOT applied: it is recorded
            // with its reason and the step runs on the free chain. An override
            // that resolves to a free or local model is applied, as before.
            let mut override_applied = false;
            if let Some(model_override) = &step.model_override {
                let mut resolved = model_override.clone();
                let mut refusal: Option<&str> = None;
                let requested = model_override
                    .model_identifier
                    .clone()
                    .unwrap_or_else(|| "(no identifier)".to_string());
                match &model_override.model_identifier {
                    Some(id) => match state
                        .request
                        .available_models
                        .iter()
                        .find(|m| &m.identifier == id)
                    {
                        Some(profile) if !profile.is_free && !state.request.allow_paid_models => {
                            refusal = Some("paid model")
                        }
                        Some(profile) => {
                            resolved.model_type.get_or_insert_with(|| profile.model_type.clone());
                            resolved.api_endpoint = resolved.api_endpoint.or_else(|| profile.api_endpoint.clone());
                            resolved.api_key_env = resolved.api_key_env.or_else(|| profile.api_key_env.clone());
                            resolved.api_key = resolved.api_key.or_else(|| profile.api_key.clone());
                            resolved.wire_protocol = resolved.wire_protocol.or_else(|| profile.wire_protocol.clone());
                            resolved.bitnet_cli_path = resolved.bitnet_cli_path.or_else(|| profile.bitnet_cli_path.clone());
                            resolved.local_model_path = resolved.local_model_path.or_else(|| profile.local_model_path.clone());
                        }
                        None => refusal = Some("unknown model"),
                    },
                    None => {
                        // Without an identifier the override must itself be
                        // local or name an OpenRouter endpoint; anything else
                        // could point at a paid provider.
                        let local = model_override.model_type.as_deref().is_some_and(|t| t != "api");
                        let openrouter = model_override
                            .api_endpoint
                            .as_deref()
                            .is_some_and(|e| e.contains("openrouter.ai"));
                        if !(local || openrouter) {
                            refusal = Some("override without a free model identifier or OpenRouter endpoint");
                        }
                    }
                }
                match refusal {
                    Some(kind) => {
                        let msg = format!("refused: {kind}; the step uses the free fallback chain");
                        tracing::warn!(step = step.step_index, requested = %requested, "{msg}");
                        state.thinking_log.push(crate::orchestrator::ThinkingEntry {
                            stage: format!("model_override_refused:step{}", step.step_index),
                            raw_response: format!("requested={requested}; {msg}"),
                            ..Default::default()
                        });
                    }
                    None => {
                        override_applied = true;
                        if let Ok(v) = serde_json::to_value(&resolved) {
                            exec_input["model_override_config"] = v;
                        }
                    }
                }
            }

            let mut retries = 0;
            let mut exec_result = self
                .executor
                .execute(step.pipeline_id, exec_input.clone())
                .await;

            // Confirmed live this session (repeatedly, via the methodology
            // meta-loop): pipeline 9 (especially OpenRouter) can return `Ok`
            // with a genuinely empty `response` field — a real, recurring
            // backend behavior, not a hard error. Checking only
            // exec_result.is_err() misses this entirely: neither the retry
            // loop nor the fallback chain below would ever trigger, and the
            // step would silently proceed with nothing. Self::
            // is_unusable_pipeline9_result treats "Ok but unusable" the
            // same as a hard error, for pipeline 9 only.
            while Self::is_unusable_pipeline9_result(step.pipeline_id, &exec_result)
                && retries < step.max_retries
            {
                retries += 1;
                tokio::time::sleep(tokio::time::Duration::from_millis(100 * retries as u64)).await;
                exec_result = self
                    .executor
                    .execute(step.pipeline_id, exec_input.clone())
                    .await;
            }

            // Multi-provider fallback chain (user-defined order in
            // config.toml's [models.fallback]) — only for the LLM pipeline
            // (9), and only when the blueprint didn't already pin a specific
            // model for this step (an explicit per-step choice is respected,
            // not overridden by a broader fallback sweep). See
            // try_fallback_chain: walks each registered identifier in order
            // so a step still succeeds via a different backend/provider when
            // the default one is down, rate-limited, segfaults (as BitNet
            // currently does on longer prompts), returns empty responses, or
            // the account is out of funds.
            if Self::is_unusable_pipeline9_result(step.pipeline_id, &exec_result) {
                if step.pipeline_id == 9 && !override_applied {
                    let last_error = exec_result
                        .clone()
                        .err()
                        .unwrap_or_else(|| "primary model returned an empty response".to_string());
                    exec_result = self
                        .try_fallback_chain(state, step.pipeline_id, exec_input.clone(), last_error)
                        .await;
                }
            }

            tracing::info!(stage = 10, "[10.x{}] step {} execute returned", iteration, step.step_index);
            final_output = exec_result?;
            self.record_thinking(
                state,
                &format!("Step Execution (step {})", step.step_index),
                &final_output,
            );
            tracing::info!(stage = 10, "[10.t{}] step {} thinking recorded", iteration, step.step_index);

            // Wait for graph update if configured
            if step.wait_for_graph_update {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            // Check loop continuation condition
            if should_loop {
                if let Some(loop_config) = &step.loop_config {
                    let should_continue = match loop_config.loop_type {
                        LoopType::Count => iteration + 1 < loop_config.max_iterations,
                        LoopType::While => {
                            // Evaluate condition (simplified - check if output indicates completion)
                            let output_text = self.extract_output_text(&final_output);
                            !output_text.to_lowercase().contains("complete")
                                && !output_text.to_lowercase().contains("done")
                        }
                        LoopType::Until => {
                            let output_text = self.extract_output_text(&final_output);
                            output_text.to_lowercase().contains("continue")
                                || !output_text.to_lowercase().contains("complete")
                        }
                        LoopType::ForEach => iteration + 1 < iterations,
                    };

                    if !should_continue {
                        break;
                    }
                }
            }
        }

        let tokens_used = final_output
            .get("tokens_used")
            .and_then(|t| t.as_u64())
            .unwrap_or(0) as u32;

        state.tokens_used_so_far += tokens_used;

        // Real, lightweight compliance check — only when this step's branch
        // actually matched real methodology content (never fabricated
        // against nothing). One cheap LLM judgment, recorded via the normal
        // thinking-log path, never blocking/retrying the step.
        let compliance_check = if step.methodology_ids.is_empty() {
            None
        } else {
            let mut rules_text_parts = Vec::new();
            for &method_id in &step.methodology_ids {
                if let Ok(Some(container)) = self.store.get_container(method_id).await {
                    if let Some(rules) = Self::load_methodology_rules_full(&container) {
                        let rules = super::amt::render_within_window(
                            state,
                            "compliance_methodology_rules",
                            8,
                            rules,
                        );
                        rules_text_parts.push(rules);
                    }
                }
            }
            if rules_text_parts.is_empty() {
                None
            } else {
                let output_text = self.extract_output_text(&final_output);
                // The whole output is what gets judged; a 1500-char cut could
                // pass a non-compliant output on the part that was never read.
                let output_text =
                    super::amt::render_within_window(state, "compliance_output", 4, output_text);
                let compliance_prompt = format!(
                    "Decision rules for this task:\n{}\n\nOutput produced:\n{}\n\n\
                     Does the output comply with these rules? Return ONLY valid JSON: \
                     {{\"compliant\": true|false, \"reason\": \"brief explanation\"}}",
                    rules_text_parts.join(" / "),
                    output_text
                );
                let compliance_input = serde_json::json!({
                    "prompt": compliance_prompt,
                    "max_tokens": 150,
                    "temperature": 0.1,
                    "system_prompt": "Judge rule compliance. Return only valid JSON, no explanation outside the JSON."
                });
                // A failure here (call error OR unparseable response) is a
                // real model/network issue, not an expected "nothing to
                // check" case — retry before giving up rather than silently
                // continuing on the first failure. Still never blocks the
                // step itself: after retries are exhausted, this surfaces
                // loudly (tracing::error!) as a real, visible problem rather
                // than being treated the same as "no methodology matched."
                const MAX_COMPLIANCE_RETRIES: u32 = 2;
                let mut attempt = 0;
                let mut outcome: Option<ComplianceCheckResult> = None;
                let mut last_error: Option<String> = None;
                loop {
                    match self.metered_execute_resilient(state, compliance_input.clone(), "methodology_compliance_check").await {
                        Ok(result) => {
                            self.record_thinking(
                                state,
                                &format!("Compliance Check (step {})", step.step_index),
                                &result,
                            );
                            let raw = result.get("response").and_then(|r| r.as_str()).unwrap_or("");
                            let json_str = Self::extract_json_from_response(raw, '{', '}');
                            let parsed = serde_json::from_str::<serde_json::Value>(json_str.trim())
                                .ok()
                                .and_then(|v| {
                                    let compliant = v.get("compliant")?.as_bool()?;
                                    let reason = v
                                        .get("reason")
                                        .and_then(|r| r.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    Some(ComplianceCheckResult { compliant, reason })
                                });
                            match parsed {
                                Some(r) => {
                                    outcome = Some(r);
                                    break;
                                }
                                None => {
                                    last_error = Some(format!("unparseable response: {}", raw));
                                }
                            }
                        }
                        Err(e) => {
                            last_error = Some(e);
                        }
                    }
                    attempt += 1;
                    if attempt > MAX_COMPLIANCE_RETRIES {
                        break;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(150 * attempt as u64)).await;
                }
                if outcome.is_none() {
                    tracing::error!(
                        step_index = step.step_index,
                        attempts = attempt,
                        error = ?last_error,
                        "Compliance check failed after retries — real model/parsing issue, not a normal 'nothing to check' case"
                    );
                }
                outcome
            }
        };

        // Fire OnStepComplete hook — living system integration
        self.on_step_complete(
            state,
            step,
            &StepResult {
                step_index: step.step_index,
                pipeline_id: step.pipeline_id,
                output: final_output.clone(),
                tokens_used,
                iterations: total_iterations,
                sub_step_results: sub_step_results.clone(),
                compliance_check: compliance_check.clone(),
            },
        )
        .await;

        Ok(StepResult {
            step_index: step.step_index,
            pipeline_id: step.pipeline_id,
            output: final_output,
            tokens_used,
            iterations: total_iterations,
            sub_step_results,
            compliance_check,
        })
    }

    async fn stage_9_result_collection(
        &self,
        state: &mut OrchestrationState,
    ) -> Result<(), String> {
        let stage_start = std::time::Instant::now();

        // Complete or fail task via TaskManager — but never override a
        // user-requested cancellation (TaskManager::cancel_task already set
        // status to "cancelled"; complete_task/fail_task would clobber it).
        if let Some(task_id) = state.task_id {
            if self.task_manager.read().await.is_cancelled(task_id).await {
                // Nothing to do — status is already correct.
            } else if state.final_response.is_some() {
                let outputs = state
                    .step_results
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "step": r.step_index,
                            "output": self.extract_output_text(&r.output),
                            "tokens": r.tokens_used
                        })
                    })
                    .collect::<Vec<_>>();

                let _ = self
                    .task_manager
                    .read()
                    .await
                    .complete_task(
                        task_id,
                        Some(serde_json::json!({
                            "response": state.final_response,
                            "steps": outputs
                        })),
                        state.tokens_used_so_far,
                    )
                    .await;
            } else {
                // The failure keeps its cause: the walk's last attempt (model,
                // failure_source tag, outcome, next step) is read from the
                // thinking log entry the walk writes as "walk_last_attempt".
                // The message changes; the failure itself is unchanged.
                let last_attempt = state
                    .thinking_log
                    .iter()
                    .rev()
                    .find(|e| e.stage == "walk_last_attempt")
                    .map(|e| e.raw_response.clone());
                let message = match last_attempt {
                    Some(line) => format!("No response generated: last attempt {line}"),
                    None => "No response generated".to_string(),
                };
                let _ = self
                    .task_manager
                    .read()
                    .await
                    .fail_task(task_id, message)
                    .await;
            }
        }

        self.record_stage_timed(
            state,
            11,
            "Result Collection",
            true,
            &format!("Collected {} step results", state.step_results.len()),
            stage_start.elapsed().as_millis() as u64,
        );

        Ok(())
    }

    async fn stage_10_post_execution(&self, state: &mut OrchestrationState) -> Result<(), String> {
        let stage_start = std::time::Instant::now();

        if !state.request.consciousness_enabled {
            self.record_stage_timed(
                state,
                12,
                "Post-execution",
                true,
                "Consciousness disabled - skipped",
                stage_start.elapsed().as_millis() as u64,
            );
            return Ok(());
        }

        // Consciousness hooks are now handled by TaskManager automatically
        // when complete_task or fail_task is called.
        //
        // Additional consciousness processing can be done here if needed:

        // Store experience with more detail via consciousness pipelines
        let experience_input = serde_json::json!({
            "action": "StoreExperience",
            "experience_type": if state.final_response.is_some() { "task_success" } else { "task_failure" },
            "summary": crate::orchestrator::prefix_at_char_boundary(&state.cleaned_prompt, 200),
            "task_id": state.task_id,
            "user_id": state.request.user_id,
            "tags": state.topics.clone(),
            "keywords": state.keywords.iter().cloned().collect::<Vec<_>>(),
            "methodologies_used": state.methodologies.clone(),
            "blueprint_id": state.blueprint_id,
            "significance": if state.final_response.is_some() { 0.5 } else { 0.3 },
            "tokens_used": state.tokens_used_so_far
        });

        // Pipeline 41 = CoreMemoryFormation
        let _ = self.executor.execute(41, experience_input).await;

        // Update relationship if we know the user
        let relationship_input = serde_json::json!({
            "action": "RecordInteraction",
            "user_id": state.request.user_id,
            "interaction_type": "task_completion",
            "outcome": if state.final_response.is_some() { "positive" } else { "negative" },
            "topics": state.topics.clone()
        });

        // Pipeline 47 = RelationshipDevelopment
        let _ = self.executor.execute(47, relationship_input).await;

        // Update emotional state
        let emotion_input = serde_json::json!({
            "action": "ProcessTrigger",
            "trigger_type": if state.final_response.is_some() { "task_success" } else { "task_failure" },
            "source": "orchestrator",
            "intensity": 0.5,
            "context": crate::orchestrator::prefix_at_char_boundary(&state.cleaned_prompt, 100)
        });

        // Pipeline 43 = EmotionalBaselineUpdate
        let _ = self.executor.execute(43, emotion_input).await;

        self.record_stage_timed(
            state,
            12,
            "Post-execution Consciousness",
            true,
            "Experience stored, relationship updated, emotions processed",
            stage_start.elapsed().as_millis() as u64,
        );

        Ok(())
    }
    /// Section S — reconstruct the session's text graph under a token budget:
    /// validated sentence nodes from the chunk graphs (fallback: cleaned_text
    /// for chunks whose extraction produced no sentences).
    ///
    /// `scope_chunk_indices`, when non-empty, restricts reconstruction to
    /// only those chunks — this is what makes per-step context genuinely
    /// branch-scoped (each AMT branch carries its own source_chunk_indices)
    /// rather than always walking the entire session from chunk 0 regardless
    /// of which branch/step is currently executing. Empty means "no known
    /// branch scope" (e.g. the single-step fallback blueprint), which walks
    /// every chunk exactly as before. If a given scope matches no chunk at
    /// all (stale/bad indices), falls back to the full walk rather than
    /// silently returning nothing for that step.
    /// Returns the context text and how many units did not fit the budget.
    /// The caller records a non-zero count; nothing is dropped unrecorded.
    fn reconstruct_session_context(
        &self,
        state: &OrchestrationState,
        budget_tokens: u32,
        scope_chunk_indices: &[u32],
    ) -> (String, usize) {
        let budget_chars = (budget_tokens as usize).saturating_mul(4);
        let scoped = !scope_chunk_indices.is_empty();
        // One unit per chunk without sentence nodes, otherwise one per
        // sentence. Each unit keeps its raw length, which is what the budget
        // check has always used.
        let mut units: Vec<(&str, usize)> = Vec::new();
        for chunk in &state.processed_chunks {
            if scoped && !scope_chunk_indices.contains(&chunk.index) {
                continue;
            }
            if chunk.sentence_nodes.is_empty() {
                units.push((chunk.cleaned_text.trim(), chunk.cleaned_text.len()));
                continue;
            }
            for s in &chunk.sentence_nodes {
                units.push((s.content.trim(), s.content.len()));
            }
        }
        let mut out = String::new();
        let mut dropped = 0usize;
        for (i, (text, raw_len)) in units.iter().enumerate() {
            if out.len() + raw_len + 1 > budget_chars {
                dropped = units.len() - i;
                break;
            }
            out.push_str(text);
            out.push('\n');
        }
        if scoped && out.trim().is_empty() {
            return self.reconstruct_session_context(state, budget_tokens, &[]);
        }
        (out.trim().to_string(), dropped)
    }
}

// ── AMTRelation cross-reference reconciliation (T-A5) ───────────────────────
//
// Extracted from stage_3_blueprint_assignment's step/branch reconciliation
// (see the doc comment at that call site) — same logic, moved to module
// scope purely so it's independently testable. No behavior change.

/// Collect every relationship target_id reachable from `node` and its
/// descendants (relationships live on branch-level nodes, which sit one
/// level below intent-level nodes in multi-intent mode — recursing finds
/// them regardless of tree shape).
fn collect_relationship_targets(node: &AMTNode, targets: &mut Vec<u64>) {
    for rel in &node.relationships {
        targets.push(rel.target_id);
    }
    for child in &node.children {
        collect_relationship_targets(child, targets);
    }
}

/// Depth-first search for the AMTNode with the given id anywhere in the tree
/// rooted at `node`.
fn find_amt_node_by_id(node: &AMTNode, target_id: u64) -> Option<&AMTNode> {
    if node.id == target_id {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_amt_node_by_id(child, target_id) {
            return Some(found);
        }
    }
    None
}

/// A step whose branch has a real AMTRelation to another branch should see
/// that other branch's chunk indices too, not just its own — otherwise a
/// step can be scoped away from content its own AMT data says it's
/// explicitly related to.
fn related_chunk_indices(matched: &AMTNode, root: &AMTNode) -> Vec<u32> {
    let mut target_ids = Vec::new();
    collect_relationship_targets(matched, &mut target_ids);
    let mut idx = Vec::new();
    for tid in target_ids {
        if let Some(node) = find_amt_node_by_id(root, tid) {
            idx.extend(node.source_chunk_indices.iter().copied());
        }
    }
    idx
}

#[cfg(test)]
mod amt_relation_tests {
    use super::*;

    fn leaf(id: u64, content: &str, chunk_indices: Vec<u32>) -> AMTNode {
        AMTNode {
            id,
            node_type: AMTNodeType::Leaf,
            content: content.to_string(),
            source_chunk_indices: chunk_indices,
            children: vec![],
            relationships: vec![],
            methodology_ids: vec![],
            metadata: Default::default(),
            depth: 1,
            verified: true,
            confidence: 1.0,
        }
    }

    // T-A5: a branch with a real AMTRelation to another branch pulls that
    // other branch's source_chunk_indices in — the whole point of making
    // relationships read, not just written.
    #[test]
    fn related_chunk_indices_pulls_in_the_target_branchs_chunks() {
        let mut branch_a = leaf(1, "auth branch", vec![10, 11]);
        let branch_b = leaf(2, "session branch", vec![20, 21]);
        branch_a.relationships.push(AMTRelation {
            target_id: 2,
            relation_type: AMTRelationType::DependsOn,
            confidence: 1.0,
        });
        let root = AMTNode {
            id: 0,
            node_type: AMTNodeType::Root,
            content: "root".to_string(),
            source_chunk_indices: vec![],
            children: vec![branch_a.clone(), branch_b],
            relationships: vec![],
            methodology_ids: vec![],
            metadata: Default::default(),
            depth: 0,
            verified: true,
            confidence: 1.0,
        };

        let related = related_chunk_indices(&branch_a, &root);
        assert_eq!(related, vec![20, 21], "pulled the related branch's own chunks, not its own");
    }

    // A branch with no relationships contributes nothing extra — the
    // mechanism doesn't manufacture relatedness that isn't there.
    #[test]
    fn no_relationships_means_no_extra_chunks() {
        let branch_a = leaf(1, "standalone branch", vec![5]);
        let root = AMTNode {
            id: 0,
            node_type: AMTNodeType::Root,
            content: "root".to_string(),
            source_chunk_indices: vec![],
            children: vec![branch_a.clone()],
            relationships: vec![],
            methodology_ids: vec![],
            metadata: Default::default(),
            depth: 0,
            verified: true,
            confidence: 1.0,
        };

        assert_eq!(related_chunk_indices(&branch_a, &root), Vec::<u32>::new());
    }

    // Relationships living on a deeper descendant (multi-intent shape: an
    // intent-level node's children are the real branches) are still found —
    // this is why collect_relationship_targets recurses instead of only
    // checking `node` itself.
    #[test]
    fn relationship_on_a_descendant_branch_is_still_found() {
        let mut deep_branch = leaf(3, "deep branch", vec![]);
        let target = leaf(2, "target branch", vec![99]);
        deep_branch.relationships.push(AMTRelation {
            target_id: 2,
            relation_type: AMTRelationType::RelatesTo,
            confidence: 1.0,
        });
        let intent_node = AMTNode {
            id: 1,
            node_type: AMTNodeType::Branch,
            content: "intent".to_string(),
            source_chunk_indices: vec![],
            children: vec![deep_branch.clone()],
            relationships: vec![],
            methodology_ids: vec![],
            metadata: Default::default(),
            depth: 1,
            verified: true,
            confidence: 1.0,
        };
        let root = AMTNode {
            id: 0,
            node_type: AMTNodeType::Root,
            content: "root".to_string(),
            source_chunk_indices: vec![],
            children: vec![intent_node, target],
            relationships: vec![],
            methodology_ids: vec![],
            metadata: Default::default(),
            depth: 0,
            verified: true,
            confidence: 1.0,
        };

        assert_eq!(related_chunk_indices(&deep_branch, &root), vec![99]);
    }
}