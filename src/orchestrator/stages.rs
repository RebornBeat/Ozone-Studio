//! //! The orchestration stages — blueprint assignment, zero-shot simulation,
//! consciousness gate, step execution (with the on-complete hook), result
//! collection, and post-execution consciousness.

use super::*;

impl PromptOrchestrator {
    /// PROVIDER-STEP RECONCILIATION (2026-11-04; tool_binding.rs; docs/METHODOLOGY_BLUEPRINT_TOOLS_REVIEW_2026-11-04.md §3.3,
    /// operator: "provider is just a step type"). Run on EVERY blueprint (freshly designed or reused) after its steps exist:
    ///   1. model-proposed PROVIDER steps are validated against the registry's providers — an unknown provider is NOT executed:
    ///      the step falls back to an ordinary model step (pipeline 9) and the missing capability is recorded;
    ///   2. the methodologies' DECLARED per-rule providers are authoritative: each available one becomes a provider STEP inserted
    ///      immediately BEFORE the step that carries the rule (the step citing the rule, else a step of that methodology, else the
    ///      step whose text best overlaps the rule); that step `depends_on` it, so the provider's result reaches it as a
    ///      previous-step output; `rule_refs` records which rule made the step;
    ///   3. steps are renumbered contiguously and every `depends_on` is remapped;
    ///   4. everything required but unavailable (declared bindings with no provider, blueprint-reported `missing_capabilities`,
    ///      unknown providers named by the model) is recorded DURABLY (capability_gaps.json + ripple + methodology gap) instead
    ///      of only being logged. Returns the de-duplicated missing capability names.
    pub(crate) async fn reconcile_tool_bindings(
        state: &mut OrchestrationState,
        blueprint_label: &str,
        reported_missing: &[String],
    ) -> Vec<String> {
        use super::tool_binding::{providers_of, record_capability_gaps, GapInput, RegistryRow};
        let registry_entries = crate::mcp::registry_entries().await.unwrap_or_default();
        let rows: Vec<RegistryRow> = registry_entries.iter().map(|t| (t.name.clone(), t.capabilities.clone(), t.endpoint.clone())).collect();
        let providers = providers_of(&rows);
        let mut gap_inputs: Vec<GapInput> = Vec::new();
        let mut all_missing: Vec<String> = reported_missing.to_vec();

        // 0. indices are assigned by position BEFORE anything remaps by them (a repeated model index must not collapse the remap below)
        if let Some(detail) = Self::normalize_step_indices(&mut state.blueprint_steps) {
            tracing::info!(detail = %detail, "reconcile: step indices assigned by position");
        }

        // 1. validate model-proposed provider steps
        for step in state.blueprint_steps.iter_mut() {
            if !step.is_provider_step() {
                continue;
            }
            let wanted = step.provider.clone().unwrap_or_default();
            match providers.keys().find(|k| k.eq_ignore_ascii_case(&wanted)).cloned() {
                Some(id) => {
                    // a pinned tool must belong to the provider, else it is chosen at call time
                    if let Some(t) = step.tool.clone() {
                        if !providers.get(&id).map(|ts| ts.contains(&t)).unwrap_or(false) {
                            step.tool = None;
                        }
                    }
                    step.provider = Some(id);
                    step.pipeline_id = 0;
                }
                None => {
                    all_missing.push(wanted.clone());
                    gap_inputs.push(GapInput {
                        kind: "provider".to_string(),
                        name: wanted,
                        source: "blueprint".to_string(),
                        detail: format!(
                            "blueprint \"{}\" step {} named a provider that is not registered — the step runs as an ordinary model step",
                            blueprint_label, step.step_index
                        ),
                    });
                    step.step_type = "pipeline".to_string();
                    step.provider = None;
                    step.tool = None;
                    step.args = serde_json::Value::Null;
                    step.pipeline_id = 9;
                }
            }
        }

        // 2. declared bindings are authoritative → provider steps inserted before their carrier
        let words = |t: &str| -> std::collections::HashSet<String> {
            t.split(|c: char| !c.is_alphanumeric()).filter(|w| w.len() > 3).map(|w| w.to_lowercase()).collect()
        };
        let manifest = state.tool_manifest.clone();
        let mut inserts: Vec<(usize, BlueprintStep)> = Vec::new(); // (carrier position in the CURRENT vector, new provider step)
        for m in &manifest {
            let status = m["status"].as_str().unwrap_or("");
            let rule_ref = m["rule_ref"].as_str().unwrap_or("").to_string();
            let name = m["name"].as_str().unwrap_or("").to_string();
            if status == "missing" {
                if !m["optional"].as_bool().unwrap_or(false) {
                    all_missing.push(name.clone());
                }
                continue; // already recorded as a gap at Stage 3
            }
            if status != "registered" && status != "degraded" {
                continue;
            }
            let Some(provider) = m["provider"].as_str().map(String::from) else { continue };
            // a provider step for this provider already exists (model-made): adopt the rule instead of duplicating
            if let Some(st) = state.blueprint_steps.iter_mut().find(|st| st.is_provider_step() && st.provider.as_deref() == Some(provider.as_str())) {
                if !st.rule_refs.contains(&rule_ref) {
                    st.rule_refs.push(rule_ref.clone());
                }
                continue;
            }
            // one provider step per (carrier, provider) — further rules of the same carrier just add their ref
            let mid = m["methodology_id"].as_u64().unwrap_or(0);
            let rule_words = words(m["rule_text"].as_str().unwrap_or(""));
            let carrier = state
                .blueprint_steps
                .iter()
                .position(|st| !st.is_provider_step() && st.rule_refs.iter().any(|r| *r == rule_ref))
                .or_else(|| state.blueprint_steps.iter().position(|st| !st.is_provider_step() && st.methodology_ids.contains(&mid)))
                .or_else(|| {
                    state
                        .blueprint_steps
                        .iter()
                        .enumerate()
                        .filter(|(_, st)| !st.is_provider_step())
                        .map(|(i, st)| (i, words(&st.description).intersection(&rule_words).count()))
                        .filter(|(_, n)| *n > 0)
                        .max_by_key(|(_, n)| *n)
                        .map(|(i, _)| i)
                });
            let Some(pos) = carrier else {
                tracing::warn!(rule_ref = %rule_ref, provider = %provider, "declared provider binding has no blueprint step to carry it — kept in the manifest only");
                continue;
            };
            if let Some((_, ins)) = inserts.iter_mut().find(|(p, st)| *p == pos && st.provider.as_deref() == Some(provider.as_str())) {
                if !ins.rule_refs.contains(&rule_ref) {
                    ins.rule_refs.push(rule_ref.clone());
                }
                continue;
            }
            let carrying = &state.blueprint_steps[pos];
            inserts.push((
                pos,
                BlueprintStep {
                    step_index: 0, // renumbered below
                    action: "call_provider".to_string(),
                    description: format!(
                        "Call provider {} for rule {} ({}){} — for: {}",
                        provider,
                        rule_ref,
                        m["rule_text"].as_str().unwrap_or(""),
                        m["purpose"].as_str().map(|p| format!(": {p}")).unwrap_or_default(),
                        carrying.description
                    ),
                    pipeline_id: 0,
                    context_requirements: carrying.context_requirements.clone(),
                    loop_config: None,
                    sub_steps: Vec::new(),
                    depends_on: carrying.depends_on.clone(), // OLD index space, remapped below
                    wait_for_graph_update: false,
                    max_retries: 1,
                    timeout_ms: None,
                    model_override: None,
                    source_chunk_indices: carrying.source_chunk_indices.clone(),
                    methodology_ids: carrying.methodology_ids.clone(),
                    rule_refs: vec![rule_ref.clone()],
                    step_type: "provider".to_string(),
                    provider: Some(provider),
                    tool: None,
                    args: serde_json::Value::Null,
                    optional: m["optional"].as_bool().unwrap_or(false),
                },
            ));
        }

        // 3. rebuild the step list: provider steps directly before their carriers; renumber; remap depends_on
        if !inserts.is_empty() {
            let old = std::mem::take(&mut state.blueprint_steps);
            // entries: (step, old step_index if an original step, carrier original position if an inserted provider step)
            let mut entries: Vec<(BlueprintStep, Option<u32>, Option<usize>)> = Vec::new();
            let mut orig_entry_pos: std::collections::HashMap<usize, usize> = Default::default();
            for (pos, st) in old.into_iter().enumerate() {
                for (_, ins) in inserts.iter().filter(|(p, _)| *p == pos) {
                    entries.push((ins.clone(), None, Some(pos)));
                }
                orig_entry_pos.insert(pos, entries.len());
                let oi = st.step_index;
                entries.push((st, Some(oi), None));
            }
            let old_to_new: std::collections::HashMap<u32, u32> =
                entries.iter().enumerate().filter_map(|(i, (_, oi, _))| oi.map(|o| (o, i as u32))).collect();
            let provider_entries: Vec<(usize, usize)> = entries
                .iter()
                .enumerate()
                .filter_map(|(i, (_, _, carrier))| carrier.map(|c| (c, i)))
                .collect(); // (carrier original pos, provider step entry idx)
            for (i, (st, _, _)) in entries.iter_mut().enumerate() {
                st.step_index = i as u32;
                st.depends_on = st.depends_on.iter().filter_map(|d| old_to_new.get(d).copied()).collect();
            }
            for (carrier_pos, prov_idx) in provider_entries {
                if let Some(&ci) = orig_entry_pos.get(&carrier_pos) {
                    let pidx = prov_idx as u32;
                    if !entries[ci].0.depends_on.contains(&pidx) {
                        entries[ci].0.depends_on.push(pidx);
                    }
                }
            }
            state.blueprint_steps = entries.into_iter().map(|(st, _, _)| st).collect();
        }

        // 4. blueprint-reported missing capabilities
        for c in reported_missing {
            gap_inputs.push(GapInput {
                kind: "capability".to_string(),
                name: c.clone(),
                source: "blueprint".to_string(),
                detail: format!("blueprint \"{}\" reported it as a missing capability", blueprint_label),
            });
        }
        record_capability_gaps(&gap_inputs);
        all_missing.sort();
        all_missing.dedup();
        for n in &all_missing {
            if !state.missing_capabilities.iter().any(|m| m["name"].as_str() == Some(n.as_str())) {
                state.missing_capabilities.push(serde_json::json!({ "kind": "capability", "name": n, "source": "blueprint", "status": "missing" }));
            }
        }
        all_missing
    }


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
        // ORIGIN (keyword program): the files the scope is handling are known now — every methodology created from here on (base, closure,
        // meta) records them as its first-hit files; the request's usable keywords were noted at the end of Stage 2.
        crate::keyword_program::note_files(state.file_graphs.values().copied());
        self.gather_methodologies(state).await?;

        // Stage 4's wall time was recorded as 0 ms (live EU walk 2026-11-09: 90 minutes hidden) — time the whole stage (4a + 4b).
        let stage4_started = std::time::Instant::now();
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
        self.record_stage_timed(
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
            stage4_started.elapsed().as_millis() as u64,
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
        self.stage_3_validated(state).await?;

        // CAPABILITY CLOSURE (operator 2026-11-06: "we are supposed to be acting on them right then and there … we create them on the spot …
        // we increase feasibility"). A blueprint that reports missing capabilities is no longer recorded-and-carried-on: each gap is JUDGED
        // (does the request really require it?) and every REAL need is closed on the spot by drafting a methodology for it, after which the
        // plan is rebuilt from the richer methodology set. Bounded by PROGRESS, not a count: another round runs only if the last one
        // created something AND the number of capabilities still judged necessary went down.
        {
            let mut last_needed = usize::MAX;
            loop {
                let (created, needed) = self.close_capability_gaps(state).await?;
                if created == 0 || needed >= last_needed {
                    break;
                }
                last_needed = needed;
                tracing::info!(stage = 6, created, needed, "capability closure: methodologies created on the spot — re-planning from the richer set");
                state.replan = true;
                let replanned = self.stage_3_validated(state).await;
                state.replan = false;
                replanned?;
                self.work_order_after_replan(state).await;
            }
        }

        // HIT capture: the methodologies the final plan's steps actually carry were APPLIED (the strongest positive label)
        {
            let mut applied: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
            for st in &state.blueprint_steps {
                applied.extend(st.methodology_ids.iter().copied());
            }
            for id in applied {
                crate::keyword_program::record_event("applied", id, &state.keywords, serde_json::json!({"blueprint": state.blueprint_id}));
            }
        }

        // STAGE 7: Zero-Shot Simulation (with AMT traversal)
        tracing::info!(stage = 7, "Stage 7 (Zero-Shot Simulation) starting — this calls the configured model backend and may take a while (e.g. BitNet loading its model fresh per call)");
        tracing::info!(stage = 7, "[7.0] simulation context: methodologies in scope = {}, applicable tools = {}", state.methodologies.len(), state.applicable_tools.len());
        self.stage_4_zero_shot_simulation(state).await?;

        // POST-SIMULATION FEASIBILITY RE-PLAN (E15, 2026-11-07; operator: "we increase feasibility"). A simulation that reports LOW
        // feasibility is no longer carried straight into execution: capability gaps the simulation itself raised are closed on the spot
        // (judge → draft → methodology), the plan is rebuilt from the richer set and simulated again. Bounded by PROGRESS, not a count:
        // another round only runs while feasibility improved over the round before and is still low (low < medium < high).
        {
            let rank = |f: &str| match f.trim().to_lowercase().as_str() { "high" => 2i32, "medium" => 1, _ => 0 };
            let mut prev = state.simulation_result.as_ref().map(|r| rank(&r.overall_feasibility)).unwrap_or(2);
            while prev == 0 {
                let (created, needed) = self.close_capability_gaps(state).await?;
                // the re-plan itself is the action when no new capability could be created: it re-designs the plan with the
                // simulation's risks now in the request context; only ONE such blind re-design is allowed (marker), later rounds need a
                // capability created.
                if created == 0 {
                    if !state.closure_attempted.insert("__feasibility_replan__".to_string()) {
                        break;
                    }
                }
                tracing::info!(stage = 7, created, needed, "feasibility LOW — re-planning and re-simulating");
                state.replan = true;
                let replanned = self.stage_3_validated(state).await;
                state.replan = false;
                replanned?;
                self.work_order_after_replan(state).await;
                self.stage_4_zero_shot_simulation(state).await?;
                let now = state.simulation_result.as_ref().map(|r| rank(&r.overall_feasibility)).unwrap_or(2);
                if now <= prev {
                    break;
                }
                prev = now;
            }
        }

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
        tracing::info!(stage = 14, "Stage 8b (Jurisdiction Full Gate, stage key 14) starting");
        self.emit_marker(state, "[8b]", "");
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
        self.emit_marker(state, "[13.0]", "");
        tracing::info!(stage = 13, "[13.0] delivery context: blueprint steps executed = {}, gate decision = {:?}", state.blueprint_steps.len(), state.gate_result.as_ref().map(|g| &g.decision));
        // EDGE SUPPLY (2026-10-13): wire RelatedTo edges between the
        // containers this walk created — the next walk's seed→hop traverses
        // them. Since 2026-11-11 this runs AFTER delivery (see below). No skip:
        // a failed store call inside the pass is retried until it succeeds.
        // DELIVER FIRST (operator 2026-11-11): the answer is delivered before the link pass; the pass is a `wo_link` child item the completion
        // gate waits on (see `deliver_then_link`), retried until it succeeds in the background.
        self.deliver_then_link(state).await?;

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


    /// Persist a walk-designed blueprint: write the body file, then create a
    /// typed Container pointing at it. Returns the new container id.
    async fn persist_blueprint_container(
        &self,
        name: &str,
        state: &OrchestrationState,
        body: &serde_json::Value,
    ) -> Result<u64, String> {
        use crate::types::container::{Container, ContainerType, GlobalState, LocalState};
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let rel = format!("blueprints/walk_{}.json", nanos);
        let base = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
        let abs = std::path::Path::new(&base).join(&rel);
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("blueprint body dir: {e}"))?;
        }
        let json = serde_json::to_string_pretty(body).map_err(|e| e.to_string())?;
        std::fs::write(&abs, json).map_err(|e| format!("blueprint body write: {e}"))?;

        let mut local = LocalState::default();
        local.metadata.container_type = ContainerType::Blueprint;
        local.metadata.created_at = now;
        local.metadata.updated_at = now;
        local.metadata.provenance = "orchestrator".to_string();
        local.metadata.name = Some(name.to_string());
        local.metadata.materialized_path = Some(format!("/Blueprints/{}", name));
        local.context.keywords = state.keywords.clone();
        local.context.topics = state.topics.clone();
        local.context.methodologies = state.methodologies.clone();
        local.storage.object_store_path = Some(rel);
        let container = Container {
            global_state: GlobalState::default(),
            local_state: local,
        };
        let value = serde_json::to_value(&container).map_err(|e| e.to_string())?;
        self.store.create_container(0, value).await
    }

    /// Read a Blueprint container's body file (the JSON at
    /// storage.object_store_path, resolved under OZONE_ZSEI_DATA_DIR).
    fn read_blueprint_body(container: &serde_json::Value) -> Option<serde_json::Value> {
        let rel = container
            .get("local_state")
            .and_then(|ls| ls.get("storage"))
            .and_then(|s| s.get("object_store_path"))
            .and_then(|p| p.as_str())?;
        let base = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
        // PATH-TRAVERSAL GUARD (perimeter review 2026-10-23): object_store_path
        // is stored data — a reused/legacy blueprint carrying `../` must not
        // read outside the data dir. Resolve and require the canonical path
        // to stay under the base; anything else reads as absent.
        let abs = std::path::Path::new(&base).join(rel);
        let base_canon = std::fs::canonicalize(&base).unwrap_or_else(|_| std::path::PathBuf::from(&base));
        let abs_canon = std::fs::canonicalize(&abs).unwrap_or(abs.clone());
        if !abs_canon.starts_with(&base_canon) {
            tracing::warn!(
                requested = %rel,
                "SECURITY: blueprint body path escapes the data dir — refused (reads as absent)"
            );
            return None;
        }
        let content = std::fs::read_to_string(&abs_canon).ok()?;
        serde_json::from_str::<serde_json::Value>(&content).ok()
    }

    /// PRIOR DESIGN CONTEXT (cross-walk design mechanism 1 — the step-level
    /// half; the enrich-level half runs in build_amt_layer_by_layer): search
    /// the graph for Blueprint containers whose keywords overlap this step's
    /// description, and return their name + description as text. The model
    /// executing the step sees prior design work alongside the session
    /// replay, so it builds on prior work instead of starting from zero.
    /// Empty string when nothing matches (the common case for new topics).
    async fn load_prior_design_context(
        &self,
        state: &OrchestrationState,
        step_description: &str,
    ) -> String {
        let keywords: Vec<String> = step_description
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 3)
            .map(|w| w.to_lowercase())
            .collect(); // ALL keywords (no-cap doctrine, audit #15)
        if keywords.is_empty() {
            return String::new();
        }
        let found = self
            .store
            .search_by_keywords(&keywords, Some("Blueprint"))
            .await
            .unwrap_or_default();
        if found.is_empty() {
            return String::new();
        }
        let mut sections = Vec::new();
        for bp_id in found.iter() { // every matching prior blueprint (audit #15); the assembled step context is window-compacted downstream
            if let Ok(Some(container)) = self.store.get_container(*bp_id).await {
                let name = container
                    .get("local_state")
                    .and_then(|ls| ls.get("metadata"))
                    .and_then(|m| m.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("unnamed");
                // Content lives in the blueprint's body file, not in the
                // typed container (Context/Metadata have no description).
                let body = Self::read_blueprint_body(&container);
                let description = body
                    .as_ref()
                    .and_then(|b| b.get("description"))
                    .and_then(|d| d.as_str())
                    .unwrap_or("");
                let steps_text = body
                    .as_ref()
                    .and_then(|b| b.get("steps"))
                    .and_then(|st| st.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|st| st.get("description").and_then(|d| d.as_str()))
                            .map(|d| format!("    · {}", d))
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();
                if !description.is_empty() || !steps_text.is_empty() {
                    let mut entry = format!("- Blueprint \"{}\" (#{})", name, bp_id);
                    if !description.is_empty() {
                        entry.push_str(&format!(" — {}", description));
                    }
                    if !steps_text.is_empty() {
                        entry.push_str(&format!("\n  Prior steps:\n{}", steps_text));
                    }
                    sections.push(entry);
                }
            }
        }
        if sections.is_empty() {
            return String::new();
        }
        format!(
            "PRIOR DESIGN WORK (from previous walks on related topics — build on this, don't repeat it):\n{}",
            sections.join("\n")
        )
    }

    pub(crate) async fn file_relationship_summary(&self, state: &mut OrchestrationState) -> String {
        // MEMOIZED per request (efficiency sweep Tier 2 #9): the summary is
        // derived from state.file_graphs + the graph containers, which don't
        // change between the 3 call sites within one request — but each call
        // paid a real store round-trip per attached file. First call
        // computes; the rest read the cache. file_graphs changing mid-request
        // would be a NEW graph creation, which always re-enters through a
        // stage that recomputes anyway on its own next call — acceptable.
        if let Some(cached) = &state.file_relationship_summary_cache {
            return cached.clone();
        }
        if state.file_graphs.is_empty() {
            state.file_relationship_summary_cache = Some(String::new());
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

    /// RULES TEXT FOR A METHODOLOGY, applicability-filtered AND LABELLED: each rule is rendered as `[m<id>:r<n>] IF … THEN …` (a heuristic as
    /// `[m<id>:h<n>] WHEN …`), so a blueprint step can CITE exactly the rules it carries out (`rule_refs`) — without the label the model has nothing
    /// to cite for an ordinary rule. Rules the rule gate judged "clearly NOT applicable" are left out; everything else (including every rule never
    /// judged) is rendered. Used by every prompt built AFTER the AMT; None when the methodology has no renderable rule left.
    pub(crate) fn rules_text_for(state: &OrchestrationState, method_id: u64, container: &serde_json::Value) -> Option<String> {
        let prefix = format!("m{method_id}:");
        let body = super::tool_binding::read_methodology_body_value(container)?;
        let lines: Vec<String> = Self::rule_items(&body)
            .into_iter()
            .filter(|(k, _)| state.rule_verdicts.get(&format!("{prefix}{k}")) != Some(&false))
            .map(|(k, text)| format!("[{prefix}{k}] {text}"))
            .collect();
        if lines.is_empty() {
            None
        } else {
            Some(lines.join(" / "))
        }
    }

    /// THE RULE GATE (operator 2026-11-10) — ONE call site, ONE batched question per methodology per round, answering TWO things about its rules.
    /// WHERE: after the blueprint steps are drafted (or a stored plan is reloaded) and BEFORE `audit_rule_refs` / `reconcile_tool_bindings`. It
    /// used to run before the draft (right after the AMT); it moved here so the SAME call can also check the plan itself: both the AMT (what the
    /// request needs) and the steps (what the plan does) exist now, and provider steps / tool rows are only fixed after this gate.
    /// 1. APPLICABILITY — which rules CLEARLY do not apply to the request and its AMT. Left out only if EVERY round (strength N) says "clearly
    ///    not"; an unanswered rule is kept.
    /// 2. FIDELITY — for every (step, rule) citation the draft made: does the step actually carry out that rule? The citation is removed only if
    ///    EVERY round says it does not; an unanswered citation is kept. (A removed citation leaves the rule UNCOVERED, which `audit_rule_refs`
    ///    records for the refinement loop.)
    /// Methodologies run concurrently. Verdicts go to `state.rule_verdicts` (read by later prompts via `rules_text_for`), the tool manifest rows
    /// and the steps' `rule_refs`. A failed call can never drop a rule or a citation.
    pub(crate) async fn judge_rule_applicability(&self, state: &mut OrchestrationState) {
        if state.methodologies.is_empty() {
            return;
        }
        let request = super::amt::request_intent_text(state);
        let mut branches = String::new();
        if let Some(amt) = &state.amt {
            for c in &amt.children {
                branches.push_str(&format!("- {}\n", c.content));
                for g in &c.children {
                    branches.push_str(&format!("    - {}\n", g.content));
                }
            }
        }
        let branches = super::amt::render_within_window(state, "rule_gate_branches", 4, branches);
        let strength = crate::shared_contracts::k_validation::configured_strength();
        // steps are addressed by their POSITION in `blueprint_steps` (never by `step_index`, which a drafting model can duplicate)
        let steps_all: Vec<(usize, String, Vec<String>)> =
            state.blueprint_steps.iter().enumerate().map(|(pos, s)| (pos, s.description.clone(), s.rule_refs.clone())).collect();
        // (methodology id, name, all its rules, the keys not judged yet, the (step POSITION, rule key) citations)
        let mut work: Vec<(u64, String, Vec<(String, String)>, std::collections::BTreeSet<String>, Vec<(usize, String)>)> = Vec::new();
        for id in state.methodologies.clone() {
            let Ok(Some(container)) = self.store.get_container(id).await else { continue };
            let name = container.pointer("/local_state/metadata/name").and_then(|n| n.as_str()).unwrap_or("").to_string();
            let Some(body) = super::tool_binding::read_methodology_body_value(&container) else { continue };
            let items = Self::rule_items(&body);
            let unjudged: std::collections::BTreeSet<String> =
                items.iter().filter(|(k, _)| !state.rule_verdicts.contains_key(&format!("m{id}:{k}"))).map(|(k, _)| k.clone()).collect();
            let prefix = format!("m{id}:");
            let cites: Vec<(usize, String)> = steps_all
                .iter()
                .flat_map(|(i, _, refs)| refs.iter().filter_map(|r| r.strip_prefix(&prefix).map(|k| (*i, k.to_string()))).collect::<Vec<_>>())
                .filter(|(_, k)| items.iter().any(|(ik, _)| ik == k))
                .collect();
            if !unjudged.is_empty() || !cites.is_empty() {
                work.push((id, name, items, unjudged, cites));
            }
        }
        if work.is_empty() {
            return;
        }
        let steps_ref = &steps_all;
        let results = futures::future::join_all(
            work.iter().map(|(_, name, items, unjudged, cites)| self.rule_gate_rounds(&request, &branches, name, items, unjudged, steps_ref, cites, strength)),
        )
        .await;
        let (mut judged, mut dropped, mut cites_dropped) = (0usize, 0usize, 0usize);
        for ((id, name, items, unjudged, cites), (not_applicable, not_carried, tokens)) in work.into_iter().zip(results) {
            state.tokens_used_so_far += tokens as u32;
            for (key, text) in items.iter().filter(|(k, _)| unjudged.contains(k)) {
                let na = not_applicable.contains(key);
                let rule_ref = format!("m{id}:{key}");
                state.rule_verdicts.insert(rule_ref.clone(), !na);
                judged += 1;
                if na {
                    dropped += 1;
                }
                crate::keyword_program::record_judgment("rule_applicability", Some(id), text, &request, "does this rule clearly NOT apply to the request and its AMT?", na, strength);
                if let Some(row) = state.tool_manifest.iter_mut().find(|m| m["rule_ref"].as_str() == Some(rule_ref.as_str())) {
                    if let Some(o) = row.as_object_mut() {
                        if na {
                            o.insert("applicability".to_string(), serde_json::json!("not_applicable"));
                            o.insert("status".to_string(), serde_json::json!("not_applicable"));
                        } else {
                            o.insert("applicability".to_string(), serde_json::json!("applies"));
                        }
                    }
                }
            }
            // EVERY judged citation is a labelled sample (kept AND removed), so the ML sees both classes
            for (step_index, key) in &cites {
                let rule_text = items.iter().find(|(k, _)| k == key).map(|(_, t)| t.as_str()).unwrap_or("");
                let desc = state.blueprint_steps.get(*step_index).map(|s| s.description.clone()).unwrap_or_default();
                crate::keyword_program::record_judgment("rule_step_fidelity", Some(id), rule_text, &desc, "does this step NOT carry out the rule it cites?", not_carried.contains(&(*step_index, key.clone())), strength);
            }
            for (step_index, key) in &not_carried {
                let rule_ref = format!("m{id}:{key}");
                if let Some(st) = state.blueprint_steps.get_mut(*step_index) {
                    let before = st.rule_refs.len();
                    st.rule_refs.retain(|r| *r != rule_ref);
                    if st.rule_refs.len() < before {
                        cites_dropped += 1;
                    }
                }
            }
            if !not_applicable.is_empty() {
                tracing::info!(stage = 6, methodology = %name, left_out = not_applicable.len(), of = items.len(), "rule gate: rules judged clearly NOT applicable are left out of the plan");
                // a sound methodology has no optional rules: tell the refinement meta loop (durable record, evidence attached)
                let terms: Vec<String> = state.keywords.iter().cloned().collect();
                let judged_items: Vec<(String, String)> = items.iter().filter(|(k, _)| unjudged.contains(k)).cloned().collect();
                super::meta_loop::record_refinement_candidate(id, &name, &judged_items, &not_applicable, &terms);
            }
        }
        tracing::info!(stage = 6, methodologies = state.methodologies.len(), rules_judged = judged, left_out = dropped, citations_removed = cites_dropped, "rule gate complete (applicability + step fidelity)");
    }

    /// The rounds for ONE methodology (no state borrowed, so methodologies run concurrently). Returns (rule keys left out, (step, rule key)
    /// citations the steps do not carry out, tokens used). Both lists follow the same asymmetric rule: removed only on EVERY round's "no".
    #[allow(clippy::too_many_arguments)]
    async fn rule_gate_rounds(
        &self,
        request: &str,
        branches: &str,
        methodology: &str,
        items: &[(String, String)],
        unjudged: &std::collections::BTreeSet<String>,
        steps: &[(usize, String, Vec<String>)],
        cites: &[(usize, String)],
        strength: u32,
    ) -> (std::collections::BTreeSet<String>, std::collections::BTreeSet<(usize, String)>, u64) {
        let mut tokens = 0u64;
        let mut alive: Vec<String> = unjudged.iter().cloned().collect(); // rule keys still "clearly not" in every round so far
        let mut doubtful: std::collections::BTreeSet<(usize, String)> = cites.iter().cloned().collect(); // citations "not carried" in every round so far
        let none = || (std::collections::BTreeSet::new(), std::collections::BTreeSet::new());
        for round in 1..=strength.max(1) {
            if alive.is_empty() && doubtful.is_empty() {
                break;
            }
            // a different presentation order each round, so the votes are not the same prompt repeated
            let mut order: Vec<&(String, String)> = items.iter().collect();
            if !order.is_empty() {
                let rot = (((round - 1) as usize * order.len()) / strength.max(1) as usize).min(order.len() - 1);
                order.rotate_left(rot);
            }
            // rules are labelled by their KEY ([r0], [h1]) so one reply can name them; applicability is asked only for the not-yet-judged ones
            let rule_list = order.iter().map(|(k, t)| format!("[{k}] {t}")).collect::<Vec<_>>().join("\n");
            let ask_applicability = !alive.is_empty();
            let cited: Vec<&(usize, String)> = doubtful.iter().collect();
            let plan = steps
                .iter()
                .filter(|(i, _, _)| cited.iter().any(|(s, _)| s == i))
                .map(|(i, d, _)| format!("S{i}: {d}"))
                .collect::<Vec<_>>()
                .join("\n");
            let mut questions = String::new();
            if ask_applicability {
                let keys = alive.iter().map(|k| format!("[{k}]")).collect::<Vec<_>>().join(" ");
                questions.push_str(&format!("A. Among the rules {keys}: which CLEARLY do not apply to this request — its subject or condition has nothing to do with what the request and its branches ask for? When a rule might apply, or you are unsure, it APPLIES. Put every one of those rules in exactly one of \"applies\" / \"clearly_not\".\n"));
            }
            if !cited.is_empty() {
                let pairs = cited.iter().map(|(s, k)| format!("S{s}|{k}")).collect::<Vec<_>>().join(" ");
                questions.push_str(&format!("B. The plan's steps cite rules. For each pair (step|rule) in {pairs}: does the step, as written, actually carry out that rule — do what its THEN/action says? When it plausibly does, or you are unsure, it CARRIES. Put every pair in exactly one of \"carries\" / \"does_not_carry\".\n"));
            }
            let prompt = format!(
                "REQUEST (the user's request, whole):\n{request}\n\nWHAT THE REQUEST BREAKS DOWN INTO (the plan's branches):\n{branches}\n\nMETHODOLOGY: {methodology}\nITS RULES:\n{rule_list}\n\nTHE PLAN'S STEPS THAT CITE ITS RULES:\n{plan}\n\nQUESTIONS:\n{questions}\nReturn ONLY JSON: {{\"applies\": [rule keys], \"clearly_not\": [rule keys], \"carries\": [\"S<n>|<key>\"], \"does_not_carry\": [\"S<n>|<key>\"]}} — omit a list whose question was not asked."
            );
            let input = serde_json::json!({
                "prompt": prompt, "max_tokens": 800, "temperature": 0.1,
                "system_prompt": "You decide which methodology rules apply to a request and whether plan steps carry them out. Return only valid JSON."
            });
            let res = self.executor.execute(9, input).await;
            let parsed = match &res {
                Ok(r) => {
                    if let Some(t) = r.get("tokens_used").and_then(|x| x.as_u64()) {
                        tokens += t;
                    }
                    let text = r.get("response").and_then(|x| x.as_str()).unwrap_or("");
                    if text.trim().is_empty() {
                        None
                    } else {
                        serde_json::from_str::<serde_json::Value>(PromptOrchestrator::extract_json_object_shared(text).trim()).ok()
                    }
                }
                Err(_) => None,
            };
            let Some(v) = parsed else {
                let (a, b) = none();
                return (a, b, tokens); // a failed call can never drop a rule or a citation
            };
            let strs = |key: &str| -> std::collections::BTreeSet<String> {
                v.get(key).and_then(|a| a.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.trim().trim_matches(|c| c == '[' || c == ']').to_string())).collect()).unwrap_or_default()
            };
            if ask_applicability {
                let (applies, clearly_not) = (strs("applies"), strs("clearly_not"));
                // kept = said "applies", said both, or was not answered; only a clear single "clearly_not" goes on to the next round
                alive.retain(|k| clearly_not.contains(k) && !applies.contains(k));
            }
            if !cited.is_empty() {
                let (carries, not_carry) = (strs("carries"), strs("does_not_carry"));
                doubtful.retain(|(s, k)| {
                    let label = format!("S{s}|{k}");
                    not_carry.contains(&label) && !carries.contains(&label)
                });
            }
        }
        // what is left said "clearly not" / "does not carry" in EVERY round
        (alive.into_iter().collect(), doubtful, tokens)
    }

    /// THE BLUEPRINT GATE (deterministic, no model): after the steps are final, every cited `rule_ref` must exist and must not be a rule the
    /// gate judged inapplicable. Invalid and inapplicable citations are removed from the step (counted and logged); the rules the steps DO
    /// cite are recorded as applied (rule-level evidence for later learning).
    pub(crate) fn audit_rule_refs(state: &mut OrchestrationState) {
        let known: std::collections::HashSet<String> = state.rule_verdicts.keys().cloned().collect();
        let manifest_refs: std::collections::HashSet<String> = state.tool_manifest.iter().filter_map(|m| m["rule_ref"].as_str().map(String::from)).collect();
        let (mut invalid, mut inapplicable, mut cited) = (0usize, 0usize, 0usize);
        let mut applied: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for st in state.blueprint_steps.iter_mut() {
            let before = st.rule_refs.len();
            let verdicts = &state.rule_verdicts;
            st.rule_refs.retain(|r| {
                let exists = known.contains(r) || manifest_refs.contains(r);
                if !exists {
                    invalid += 1;
                    return false;
                }
                if verdicts.get(r) == Some(&false) {
                    inapplicable += 1;
                    return false;
                }
                applied.insert(r.clone());
                true
            });
            cited += st.rule_refs.len().min(before);
        }
        if invalid + inapplicable > 0 {
            tracing::warn!(invalid, inapplicable, "blueprint gate: removed rule citations that do not exist / were judged inapplicable");
        }
        for r in &applied {
            if let Some(mid) = r.strip_prefix('m').and_then(|x| x.split(':').next()).and_then(|x| x.parse::<u64>().ok()) {
                crate::keyword_program::record_event("rule_cited", mid, &state.keywords, serde_json::json!({"kind": "rule", "rule_ref": r})); // its own kind: never inflates the methodology-level `applied` counter
            }
        }
        // COVERAGE (operator 2026-11-10: "a sound methodology is one where all rules apply"): every rule judged applicable should be carried by
        // at least one step. Deterministic; a rule no step cites is RECORDED (never added, never removed) for the refinement loop — either the
        // plan missed an applicable rule or the rule is not really actionable as a step.
        // (declared per-rule PROVIDER rules are excluded: `reconcile_tool_bindings` runs right after this audit and attaches each one to a carrier or
        // inserts its provider step, so counting them here would report rules that are about to be covered)
        let uncovered: Vec<String> =
            state.rule_verdicts.iter().filter(|(r, ok)| **ok && !applied.contains(*r) && !manifest_refs.contains(*r)).map(|(r, _)| r.clone()).collect();
        if !uncovered.is_empty() && !state.blueprint_steps.is_empty() {
            tracing::warn!(uncovered = uncovered.len(), of = state.rule_verdicts.len(), "blueprint gate: applicable rules no step cites");
            crate::orchestrator::meta_loop::record_uncovered_rules(&uncovered, &state.keywords);
            for r in &uncovered {
                if let Some(mid) = r.strip_prefix('m').and_then(|x| x.split(':').next()).and_then(|x| x.parse::<u64>().ok()) {
                    crate::keyword_program::record_event("rule_uncovered", mid, &state.keywords, serde_json::json!({"kind": "rule", "rule_ref": r}));
                }
            }
        }
        tracing::info!(cited, distinct_rules_applied = applied.len(), "blueprint gate: rule citations audited");
    }

    /// CAPABILITY CLOSURE, step 1 + 2 + 3 (see the call site in the walk). Returns `(things_created, gaps_judged_necessary)` — a methodology or
    /// a tool counts once per gap.
    /// 1. JUDGE every still-unjudged missing capability: "can the request be fully and correctly completed WITHOUT it?" — strength-5
    ///    consecutive YES (the assured action is DROPPING a requirement, so that is what needs the unanimity). Dropped gaps are marked
    ///    `not_needed` in capability_gaps.json (reason kept) and leave `state.missing_capabilities`, so the simulation no longer plans
    ///    around tools the request never needed (the live walk invented a "representative_user_validation_panel" for a 2-sentence answer).
    /// 2. For every gap still judged NECESSARY: draft a methodology that achieves it on the spot (`draft_methodology_for_capability`).
    /// 3. And create the missing TOOL (`create_tool_for_capability`, E30) — automatic; the finding is published (see `tool_creation.rs`).
    pub(crate) async fn close_capability_gaps(&self, state: &mut OrchestrationState) -> Result<(usize, usize), String> {
        let pending: Vec<String> = state
            .missing_capabilities
            .iter()
            .filter(|m| m["status"].as_str().unwrap_or("missing") == "missing")
            .filter_map(|m| m["name"].as_str().map(|n| n.to_string()))
            .filter(|n| !state.closure_attempted.contains(&n.to_lowercase()))
            .collect();
        // one capability can be reported by several gap rows / steps — judge and draft it ONCE (live walk 1154 drafted three
        // methodologies for the same "code_callgraph")
        let pending: Vec<String> = {
            let mut seen = std::collections::HashSet::new();
            pending.into_iter().filter(|n| seen.insert(n.to_lowercase())).collect()
        };
        if pending.is_empty() {
            return Ok((0, 0));
        }
        let request = state.request.prompt.clone();
        let plan = state
            .blueprint_steps
            .iter()
            .map(|st| format!("{}. {} — {}", st.step_index, st.action, st.description))
            .collect::<Vec<_>>()
            .join("\n");
        let mut needed: Vec<String> = Vec::new();
        // the necessity judgments are INDEPENDENT of each other → prompts first, votes concurrently, results applied in order
        let mut judge: Vec<(String, String)> = Vec::new(); // (capability name, prompt)
        for name in pending {
            state.closure_attempted.insert(name.to_lowercase());
            let prompt = format!(
                "REQUEST (the user's request, whole):\n{request}\n\nTHE PLAN'S STEPS:\n{plan}\n\nCAPABILITY the plan reports as MISSING: \"{name}\"\n\nQUESTION: Can the request above be fully and correctly completed WITHOUT this capability? Answer YES if it is not genuinely necessary — i.e. it only adds thoroughness, process, validation or polish beyond what the request actually asks. Answer NO only if the request cannot be completed correctly without it."
            );
            judge.push((name, prompt));
        }
        let strength = crate::shared_contracts::k_validation::configured_strength();
        let verdicts = self.confirm_yes_no_batch(state, judge.iter().map(|j| j.1.clone()).collect(), strength).await;
        for ((name, _prompt), droppable) in judge.into_iter().zip(verdicts.into_iter()) {
            crate::keyword_program::record_judgment("capability_necessity", None, &name, &request, "can the request be completed WITHOUT this capability?", droppable, strength);
            if droppable {
                state.missing_capabilities.retain(|m| m["name"].as_str().map(|n| !n.eq_ignore_ascii_case(&name)).unwrap_or(true));
                super::tool_binding::set_gap_status(&name, "not_needed", "judged not required to fulfil the request (strength-5 yes/no)", "capability-closure");
                tracing::info!(capability = %name, "capability closure: judged NOT required — dropped from the plan's missing list");
            } else {
                needed.push(name);
            }
        }
        let mut created = 0usize;
        let needed_count = needed.len();
        for name in needed {
            // NO SKIP (operator 2026-11-11): a capability judged NECESSARY is closed or the walk PAUSES and retries — "could not be drafted this walk" is not
            // an outcome. On a failure the name is taken back out of `closure_attempted` and the error is returned, so the resume runs the closure for it again.
            let already_drafted = super::tool_binding::capability_gaps()
                .iter()
                .any(|g| g["name"].as_str().map(|n| n.eq_ignore_ascii_case(&name)).unwrap_or(false) && g["status"].as_str() == Some("methodology_drafted"));
            let mut drafted_id: Option<u64> = None;
            if !already_drafted {
                match self.draft_methodology_for_capability(state, &name).await {
                    Ok(id) => {
                        drafted_id = Some(id);
                        state.methodologies.push(id);
                        created += 1;
                        super::tool_binding::set_gap_status(&name, "methodology_drafted", &format!("methodology {id} created on the spot"), "capability-closure");
                        tracing::info!(capability = %name, methodology_id = id, "capability closure: methodology created on the spot");
                    }
                    Err(e) => {
                        super::tool_binding::set_gap_status(&name, "wanted", &format!("judged necessary; its methodology could not be drafted — the walk pauses and retries ({e})"), "capability-closure");
                        state.closure_attempted.remove(&name.to_lowercase());
                        return Err(e);
                    }
                }
            }
            // TOOL CREATION ON THE IDENTIFIED NEED (operator 2026-11-10, E30): the methodology says HOW; the missing tool is created too, and the
            // created tool is published as a FINDING. A created tool counts as something created (the plan is rebuilt with it registered).
            match self.create_tool_for_capability(state, &name, drafted_id).await {
                Ok(Some(_)) => {
                    if drafted_id.is_none() {
                        created += 1; // a methodology that was created already counted
                    }
                }
                Ok(None) => {} // by design: tool creation switched off by the operator, or a tool of that name already exists
                Err(e) => {
                    state.closure_attempted.remove(&name.to_lowercase());
                    return Err(e);
                }
            }
            // the gap leaves the walk's missing list only when BOTH its methodology and its tool step are done
            state.missing_capabilities.retain(|m| m["name"].as_str().map(|n| !n.eq_ignore_ascii_case(&name)).unwrap_or(true));
        }
        Ok((created, needed_count))
    }

    /// TOOL CREATION ON AN IDENTIFIED NEED (E30, call site `capability_tool_draft`; module doc `tool_creation.rs` states exactly what a created
    /// tool is and what executes — nothing the host runs). ONE zero-shot call drafts the spec (additive creation, so one call — the strength-5
    /// necessity vote already ran before we got here); the draft is parsed for real (`tool_creation::parse_spec`), checked for a name collision,
    /// written into the tool's own folder, registered (stdio, never auto-started), recorded on the gap with its provenance, and handed to the
    /// marketplace bridge as a FINDING. Any failure = nothing is created (the gap stays recorded and the next walk retries); an artifact that
    /// was written but could not be registered is removed again. Returns the created tool's name.
    async fn create_tool_for_capability(&self, state: &mut OrchestrationState, capability: &str, methodology_id: Option<u64>) -> Result<Option<String>, String> {
        use super::tool_creation as tc;
        if !tc::enabled() {
            return Ok(None); // the operator's kill switch is off: by design, not a failure
        }
        let data_dir = tc::data_dir();
        let registered: Vec<(String, String)> = crate::mcp::registry_entries()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|t| (t.name.clone(), format!("- {} {}", t.name, t.capabilities.join("; "))))
            .collect();
        let registered_names: Vec<String> = registered.iter().map(|r| r.0.clone()).collect();
        let Some(name) = tc::sanitize_name(capability) else {
            tracing::info!(capability, "tool creation: the capability has no usable tool name — none created");
            return Ok(None);
        };
        if let Some(why) = tc::collision(&name, &registered_names, &data_dir) {
            tracing::info!(capability, reason = %why, "tool creation: a tool of that name already exists — nothing to create");
            return Ok(None);
        }
        let request = state.request.prompt.clone();
        let methodology = match methodology_id {
            Some(id) => super::meta_loop::methodology_whole(&self.store, id).await.map(|(n, _kws, body)| format!("{n}\n{body}")),
            None => None,
        };
        let prompt = tc::draft_prompt(
            &crate::orchestrator::run_spans::now_line(),
            capability,
            &request,
            &registered.iter().map(|r| r.1.clone()).collect::<Vec<_>>(),
            methodology.as_deref(),
        );
        let input = serde_json::json!({
            "prompt": prompt, "max_tokens": state.model_context_limit / 4, "temperature": 0.2,
            "system_prompt": "Specify real, honest, general tools only. Return only valid JSON."
        });
        // NO SKIP (operator 2026-11-11): every failure below is returned (the walk pauses and the resume retries the closure for this capability); nothing is
        // logged-and-forgotten. Only the operator's switch and an existing tool of the same name are "by design" no-ops (Ok(None) above).
        let result = self
            .metered_execute_resilient(state, input, "capability_tool_draft")
            .await
            .map_err(|e| format!("tool draft call for '{capability}' failed: {e}"))?;
        let text = self.extract_output_text(&result);
        let parsed = serde_json::from_str::<serde_json::Value>(Self::extract_json_object_shared(&text).trim())
            .map_err(|e| format!("tool draft for '{capability}' was not valid JSON: {e}"))?;
        let spec = tc::parse_spec(&parsed, capability).map_err(|e| format!("tool draft for '{capability}' was not a real tool specification: {e}"))?;
        let created: Option<String> = {
            {
                let origin = serde_json::json!({
                    "tool": spec.name, "gap": capability, "request": request, "methodology_id": methodology_id, "task_id": state.task_id,
                    "created_by": "capability-closure", "created_at": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
                    "transport": "stdio", "auto_start": false,
                });
                match tc::write_artifact(&data_dir, &spec, &origin) {
                    Ok(art) => match crate::mcp::register_global(tc::registry_row(&spec)).await {
                        Some(_) => {
                            let mut origin = origin;
                            origin["body_path"] = serde_json::json!(art.body_path);
                            origin["body_hash"] = serde_json::json!(art.body_hash);
                            origin["implementation_path"] = serde_json::json!(art.implementation_path);
                            super::tool_binding::mark_gap_tool_created(capability, origin.clone(), "capability-closure");
                            crate::keyword_program::record_event("tool_created", methodology_id.unwrap_or(0), &state.keywords, serde_json::json!({"tool": spec.name, "gap": capability}));
                            // PUBLISH PRIVACY CHECK (operator 2026-11-11: what may leave is a rule set — `tool_creation::PUBLISH_RULES`, the same text as the
                            // methodology draft 30_tool-publish-privacy). Deterministic scan first (real matching), then ONE strength-N yes/no vote; the
                            // assured action is PUBLISHING, so every vote must say YES. A withheld finding leaves the tool created + registered locally;
                            // only the announcement is withheld, and the reason is recorded on the gap (an event, so the work-order tree shows it).
                            let body_text = tc::body_json(&spec);
                            let findings = tc::privacy_findings(&body_text, &spec.implementation, &request, &data_dir);
                            let strength = crate::shared_contracts::k_validation::configured_strength();
                            let publish_ok = if findings.is_empty() {
                                let votes = self.confirm_yes_no_batch(state, vec![tc::publish_vote_prompt(&body_text, &spec.implementation)], strength).await;
                                let ok = votes.first().copied().unwrap_or(false);
                                crate::keyword_program::record_judgment("tool_publish_privacy", methodology_id, capability, &request, "is the created tool's body safe to publish to other instances?", ok, strength);
                                ok
                            } else {
                                false
                            };
                            if publish_ok {
                                // the FINDING: a manifest (name + body hash + keywords) goes to the marketplace bridge — a no-op while publishing is off
                                crate::network::publish_created_tool(crate::network::CreatedToolFinding {
                                    name: spec.name.clone(),
                                    body_hash: art.body_hash,
                                    keywords: spec.capabilities.clone(),
                                    body_path: art.body_path,
                                });
                                tracing::info!(capability, tool = %spec.name, "tool creation: tool created, registered (stdio, never auto-started) and published as a finding");
                            } else {
                                let why = if findings.is_empty() { "the strength-N publish vote did not unanimously say the body is safe to publish".to_string() } else { findings.join("; ") };
                                super::tool_binding::note_gap_event(capability, "publish_withheld", &why, "tool-publish-privacy");
                                crate::keyword_program::record_event("tool_publish_withheld", methodology_id.unwrap_or(0), &state.keywords, serde_json::json!({"tool": spec.name, "gap": capability, "reason": why}));
                                tracing::warn!(capability, tool = %spec.name, reason = %why, "tool creation: tool created and registered locally; publishing WITHHELD by the privacy rules");
                            }
                            Some(spec.name)
                        }
                        None => {
                            tc::remove_artifact(&data_dir, &spec.name); // the registry is not installed: do not leave an unregistered artifact behind
                            return Err(format!("tool creation for '{capability}': the MCP registry is not installed — the artifact was removed; the walk pauses and retries"));
                        }
                    },
                    Err(e) => {
                        return Err(format!("tool creation for '{capability}': the artifact could not be written: {e}"));
                    }
                }
            }
        };
        crate::keyword_program::record_judgment("capability_tool_draft", methodology_id, capability, &request, "was a real tool created for this capability?", created.is_some(), 1);
        Ok(created)
    }

    /// Draft ONE real methodology that achieves `capability` for this request using the registered providers/tools and ordinary model
    /// reasoning (placeholders rejected), and persist it through pipeline 12 `Create` (decision rules carry their `requires` providers, so a
    /// provider that truly does not exist is still recorded as wanted). Returns the new methodology id.
    async fn draft_methodology_for_capability(&self, state: &mut OrchestrationState, capability: &str) -> Result<u64, String> {
        let tool_lines = state
            .applicable_tools
            .iter()
            .filter_map(|t| {
                let name = t.get("name").and_then(|n| n.as_str())?;
                let caps = t.get("capabilities").map(|c| c.to_string()).unwrap_or_default();
                Some(format!("- {name} {caps}"))
            })
            .collect::<Vec<_>>()
            .join("\n");
        let prompt = format!(
            "{}\n\nA plan for the request below needs the capability \"{capability}\", which is not installed.\n\nREQUEST (whole):\n{}\n\nREGISTERED PROVIDERS / TOOLS:\n{tool_lines}\n\nDraft ONE real methodology: concrete, actionable guidance for achieving this capability for requests like this one using ONLY the registered providers/tools above and ordinary model reasoning. A decision rule that needs a provider must declare it under \"requires\" (kind \"provider\", name, purpose). If it truly cannot be achieved without installing something not listed, include a rule whose requires names that provider.\n\nKEYWORD RULES (the \"keywords\" you give are validated one by one before they are stored): {}\n\nReturn ONLY valid JSON: {{\"name\": \"...\", \"description\": \"...\", \"principles\": [{{\"name\": \"...\", \"description\": \"...\", \"priority\": 5}}], \"heuristics\": [{{\"condition\": \"...\", \"action\": \"...\"}}], \"decision_rules\": [{{\"name\": \"...\", \"condition\": \"...\", \"outcome\": \"...\", \"requires\": []}}], \"keywords\": [\"...\"], \"topics\": [\"...\"]}}",
            crate::orchestrator::run_spans::now_line(),
            state.request.prompt,
            crate::shared_contracts::keyword_model::KEYWORD_RULES
        );
        let input = serde_json::json!({
            "prompt": prompt, "max_tokens": 800, "temperature": 0.3,
            "system_prompt": "Draft real, practical methodologies only. Return only valid JSON."
        });
        // NO SKIP (operator 2026-11-11): every failure is returned; the closure pauses the walk and the resume retries the draft
        let result = self
            .metered_execute_resilient(state, input, "capability_closure_draft")
            .await
            .map_err(|e| format!("methodology draft call for '{capability}' failed: {e}"))?;
        let text = self.extract_output_text(&result);
        let parsed: serde_json::Value = serde_json::from_str(Self::extract_json_object_shared(&text).trim())
            .map_err(|e| format!("methodology draft for '{capability}' was not valid JSON: {e}"))?;
        let real = |v: &serde_json::Value, fields: &[&str]| {
            fields.iter().any(|f| v.get(f).and_then(|x| x.as_str()).map(|s| !s.trim().is_empty() && s.trim() != "..." && s.trim() != "…").unwrap_or(false))
        };
        let has_rules = parsed.get("decision_rules").and_then(|d| d.as_array()).map(|a| a.iter().any(|e| real(e, &["condition", "outcome"]))).unwrap_or(false);
        let has_heuristics = parsed.get("heuristics").and_then(|h| h.as_array()).map(|a| a.iter().any(|e| real(e, &["condition", "action"]))).unwrap_or(false);
        if !has_rules && !has_heuristics {
            return Err(format!("methodology draft for '{capability}' was a placeholder shell, not a methodology")); // retried, never accepted
        }
        let mut keywords: Vec<String> = parsed.get("keywords").and_then(|k| serde_json::from_value(k.clone()).ok()).unwrap_or_default();
        keywords.extend(capability.split(|c: char| !c.is_alphanumeric()).filter(|w| w.len() > 2).map(|w| w.to_lowercase()));
        keywords.sort();
        keywords.dedup();
        let create = serde_json::json!({
            "action": "Create",
            "name": parsed.get("name").and_then(|n| n.as_str()).unwrap_or(capability),
            "description": parsed.get("description").and_then(|d| d.as_str()).unwrap_or(""),
            "category_id": 0,
            "principles": parsed.get("principles").cloned().unwrap_or(serde_json::json!([])),
            "heuristics": parsed.get("heuristics").cloned().unwrap_or(serde_json::json!([])),
            "decision_rules": parsed.get("decision_rules").cloned().unwrap_or(serde_json::json!([])),
            "keywords": keywords,
            "topics": parsed.get("topics").cloned().unwrap_or(serde_json::json!([])),
        });
        self.executor
            .execute(12, create)
            .await
            .map_err(|e| format!("methodology_create failed for '{capability}': {e}"))?
            .get("methodology_id")
            .and_then(|m| m.as_u64())
            .ok_or_else(|| format!("methodology_create returned no methodology_id for '{capability}'"))
    }

    /// STEP INDEX ASSIGNMENT — the single source (operator 2026-11-11: "why would a step index ever repeat … not a fail or retry … just
    /// change the step index … we don't record failures and move on"). `step_index` is assigned by POSITION, once, deterministically, at the
    /// moment steps are populated; order is preserved. `depends_on` is remapped from the old index space: an index carried by exactly one
    /// step maps to that step; an index carried by several (a drafting model repeated it) maps to the carriers positioned BEFORE the
    /// dependent step (never itself, never forward — no cycles); an index no step carries is dangling and is dropped (nothing can satisfy it).
    /// Nothing fails, nothing replans, nothing pauses, no step is dropped. Returns a description of what changed (None = already canonical).
    pub(crate) fn normalize_step_indices(steps: &mut [BlueprintStep]) -> Option<serde_json::Value> {
        let old: Vec<u32> = steps.iter().map(|s| s.step_index).collect();
        let mut carriers: std::collections::HashMap<u32, Vec<usize>> = Default::default();
        for (pos, idx) in old.iter().enumerate() {
            carriers.entry(*idx).or_default().push(pos);
        }
        let repeated: std::collections::BTreeSet<u32> = carriers.iter().filter(|(_, v)| v.len() > 1).map(|(k, _)| *k).collect();
        let mut dangling: Vec<serde_json::Value> = Vec::new();
        let mut changed_deps = false;
        for pos in 0..steps.len() {
            let mut mapped: Vec<u32> = Vec::new();
            for d in steps[pos].depends_on.clone() {
                match carriers.get(&d) {
                    Some(v) if v.len() == 1 => {
                        if v[0] != pos {
                            mapped.push(v[0] as u32);
                        }
                    }
                    Some(v) => mapped.extend(v.iter().filter(|q| **q < pos).map(|q| *q as u32)),
                    None => dangling.push(serde_json::json!({"step": pos, "depends_on": d})),
                }
            }
            mapped.sort_unstable();
            mapped.dedup();
            if mapped != steps[pos].depends_on {
                changed_deps = true;
            }
            steps[pos].depends_on = mapped;
        }
        let mut renumbered = false;
        for (pos, st) in steps.iter_mut().enumerate() {
            if st.step_index != pos as u32 {
                renumbered = true;
            }
            st.step_index = pos as u32;
        }
        if !renumbered && !changed_deps && dangling.is_empty() {
            return None;
        }
        Some(serde_json::json!({"repeated_indices": repeated, "renumbered": renumbered, "dangling_dependencies_dropped": dangling}))
    }

    /// Stage 3 + step index assignment. The plan comes from `stage_3_blueprint_assignment` (stored blueprint, drafted, or salvaged); every
    /// path then gets its indices assigned HERE by position, so a repeated/missing/offset index from a model or a stored blueprint can never
    /// reach the step loop. No replan, no failure.
    async fn stage_3_validated(&self, state: &mut OrchestrationState) -> Result<(), String> {
        self.stage_3_blueprint_assignment(state).await?;
        if let Some(detail) = Self::normalize_step_indices(&mut state.blueprint_steps) {
            tracing::info!(detail = %detail, "stage 3: step indices assigned by position");
            super::amt::record_intent_event(state, "step_index_assigned", detail);
        }
        Ok(())
    }

    async fn stage_3_blueprint_assignment(
        &self,
        state: &mut OrchestrationState,
    ) -> Result<(), String> {
        let stage_start = std::time::Instant::now();
        let tools_note = format!("{} applicable tool(s)/MCP(s) carried into the blueprint", state.applicable_tools.len());
        self.emit_marker(state, "[6.tools]", &tools_note);

        // (the RULE GATE no longer runs here: it runs after the steps exist, right before `audit_rule_refs` — see `judge_rule_applicability`)

        // Search for blueprint with 100% match — skipped when the capability-closure step is RE-PLANNING with newly created methodologies
        // (a stored blueprint that matches the keywords is exactly the plan that reported the gaps).
        let blueprint_ids = if state.replan {
            Vec::new()
        } else {
            self.store
                .search_by_keywords(
                    &state.keywords.iter().cloned().collect::<Vec<_>>(), // ALL keywords (audit #15)
                    Some("Blueprint"),
                )
                .await
                .unwrap_or_default()
        };

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
                // Load blueprint steps — from the blueprint's CONTENT FILE
                // (object_store_path) when present, falling back to inline
                // storage.steps. B17 resolution (2026-09-28): the content
                // files are the canonical home; the typed storage struct
                // drops unknown keys on the round-trip. A matched blueprint
                // with ZERO loadable steps is NOT reused — it falls through
                // to a fresh design (previously it reported "Using existing
                // blueprint" with an empty plan).
                let mut loaded_steps: Vec<BlueprintStep> = Vec::new();
                // the stored body's own `missing_capabilities` (plain strings, or objects with a `name`) — a reused plan reports them exactly
                // like a freshly designed one, so every walk of the blueprint feeds the same gap path (and the work-order gap tasks)
                let mut body_missing: Vec<String> = Vec::new();
                if let Ok(Some(container)) = self.store.get_container(bp_id).await {
                    if let Some(body) = Self::read_blueprint_body(&container) {
                        body_missing = body
                            .get("missing_capabilities")
                            .and_then(|m| m.as_array())
                            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from).or_else(|| x["name"].as_str().map(String::from))).collect())
                            .unwrap_or_default();
                    }
                    let loaded = Self::read_blueprint_body(&container)
                        .and_then(|b| b.get("steps").cloned());
                    let steps_value = loaded.or_else(|| {
                        container
                            .get("local_state")
                            .and_then(|ls| ls.get("storage"))
                            .and_then(|s| s.get("steps"))
                            .cloned()
                    });
                    if let Some(steps) = steps_value {
                        loaded_steps = serde_json::from_value(steps).unwrap_or_default();
                    }
                }
                if loaded_steps.is_empty() {
                    tracing::warn!(
                        blueprint_id = bp_id,
                        "stage 3: matched blueprint has no loadable steps — designing fresh instead of reusing an empty plan"
                    );
                } else {
                    state.blueprint_id = Some(bp_id);
                    let n = loaded_steps.len();
                    state.blueprint_steps = loaded_steps;
                    // a reused plan is held to the SAME binding rules as a new one: this run's declared per-rule tools are
                    // attached, stale/unknown tool calls dropped, missing capabilities captured (tool_binding.rs)
                    self.judge_rule_applicability(state).await;
                    Self::audit_rule_refs(state);
                    let _ = Self::reconcile_tool_bindings(state, &format!("blueprint {}", bp_id), &body_missing).await;
                    self.record_stage_timed(
                        state,
                        6,
                        "Blueprint Steps Loaded",
                        true,
                        &format!("{} step(s) from blueprint {}", n, bp_id),
                        0,
                    );
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
        // file yet (most don't — see load_methodology_rules_full).
        let methodologies_desc = if state.methodologies.is_empty() {
            "  (none matched this request)".to_string()
        } else {
            let mut lines = Vec::new();
            let mut without_rules: Vec<String> = Vec::new(); // OBSERVABILITY: a methodology that contributes no rules must never be silent
            // Copy of the ids: the loop body sizes rules against `state`
            // (mutable), so it cannot keep a borrow of state.methodologies.
            for method_id in state.methodologies.clone() {
                if let Ok(Some(container)) = self.store.get_container(method_id).await {
                    let name = container
                        .get("local_state").and_then(|ls| ls.get("metadata"))
                        .and_then(|m| m.get("name")).and_then(|n| n.as_str())
                        .unwrap_or("Unknown");
                    match Self::rules_text_for(state, method_id, &container) {
                        Some(rules) => {
                            let rules = super::amt::render_within_window(
                                state,
                                "blueprint_methodology_rules",
                                8,
                                rules,
                            );
                            lines.push(format!("  - {}: {}", name, rules))
                        }
                        None => {
                            without_rules.push(format!("{method_id}:{name}"));
                            lines.push(format!("  - {} (no detailed rules on file yet)", name))
                        }
                    }
                }
            }
            tracing::info!(
                stage = 6,
                gathered = state.methodologies.len(),
                contributing_rules = state.methodologies.len().saturating_sub(without_rules.len()),
                without_rules = %without_rules.join(" | "),
                "blueprint: methodology rules rendered into the prompt"
            );
            lines.join("\n")
        };

        let branch_count = amt.children.len();
        let jurisdiction_ctx = Self::jurisdiction_summary(state);
        let file_relationships = self.file_relationship_summary(state).await;
        // REGISTERED CAPABILITIES (§11.2): compact registry summary so the
        // blueprint can route steps to tools/MCPs when the goal matches.
        let capabilities =
            Self::capability_summary_for_need(state, &state.request.prompt);
        // RULE → REQUIRED CAPABILITIES: what the gathered methodologies' rules DECLARE they need (Stage 3 manifest).
        let tool_manifest_block = super::tool_binding::manifest_block(&state.tool_manifest);
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

RULE → REQUIRED PROVIDERS (declared by the applicable methodologies' rules — AUTHORITATIVE. A PROVIDER is an MCP
server; the host turns every available one into a PROVIDER STEP that runs before the step carrying the rule. A provider
marked "missing" is NOT installed: do not invent one — list it in missing_capabilities):
{tool_manifest_block}

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
If no existing pipeline or REGISTERED CAPABILITY can handle a requirement, add it to missing_capabilities — a missing
capability is captured and turned into a methodology + a tool request; never paper over it with an unrelated tool.
STEP TYPES: every step is "pipeline" (default — runs the listed pipeline_id) or "provider" — a step that calls an MCP
PROVIDER: {{"step_type": "provider", "provider": "<provider id from the list above>", "tool": null, "args": null,
"rule_refs": ["m<id>:r<n>"], "pipeline_id": 0, ...}}. Which of the provider's tools runs is decided at call time from the
step text unless you pin "tool"; "args" are structured arguments only when the request fixes them (e.g. {{"time": "10:10"}}).
Use ONLY provider ids from the list above or REGISTERED CAPABILITIES — an unknown provider is dropped to an ordinary
model step and recorded as missing.
rule_refs: the refs — exactly as written in square brackets in APPLICABLE METHODOLOGIES, e.g. "m12:r0" — of the methodology rules this step carries out ([] if none). A ref that is not listed there is removed.
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
            "model_override": null,
            "rule_refs": [],
            "step_type": "pipeline"
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
            "model_override": null,
            "rule_refs": [],
            "step_type": "pipeline"
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
        // No silent `{}`: a missing/unparseable blueprint is a FAILED stage, not a
        // "Generated Blueprint" with no steps that reconciliation then invents around.
        let response = Self::response_text(&bp_result).ok_or_else(|| {
            "Blueprint Assignment: the model returned no response text — not substituting an empty blueprint".to_string()
        })?;
        let bp_json = Self::parse_json_object_strict(response)
            .map_err(|e| format!("Blueprint Assignment: {e}"))?;

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
            // Captured durably below (reconcile_tool_bindings → capability_gaps.json + meta ripple + methodology gap);
            // this line is only the immediate log.
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
                        // NO SKIP (operator 2026-11-11): a step that does not deserialize strictly is rebuilt leniently from the fields it
                        // does carry — its text is kept, never dropped; the next stages (rule gate, reconciliation) judge it like any step.
                        tracing::warn!(index = i, error = %e, "Blueprint assignment: step did not deserialize strictly — rebuilt leniently from its own fields (not skipped)");
                        let text = |k: &str| step_json.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let description = {
                            let d = text("description");
                            if d.is_empty() { step_json.to_string() } else { d }
                        };
                        let action = {
                            let a = text("action");
                            if a.is_empty() { "execute_prompt".to_string() } else { a }
                        };
                        let pipeline_id = step_json.get("pipeline_id").and_then(|v| v.as_u64()).unwrap_or(9);
                        let depends_on: Vec<u32> = step_json
                            .get("depends_on")
                            .and_then(|v| v.as_array())
                            .map(|a| a.iter().filter_map(|x| x.as_u64().map(|n| n as u32)).collect())
                            .unwrap_or_default();
                        steps.push(BlueprintStep {
                            step_index: step_json.get("step_index").and_then(|v| v.as_u64()).unwrap_or(i as u64) as u32,
                            action,
                            description,
                            pipeline_id,
                            context_requirements: vec!["full_context".to_string()],
                            loop_config: None,
                            sub_steps: Vec::new(),
                            depends_on,
                            wait_for_graph_update: false,
                            max_retries: 1,
                            timeout_ms: None,
                            model_override: None,
                            source_chunk_indices: Vec::new(),
                            methodology_ids: Vec::new(),
                            rule_refs: Vec::new(),
                            step_type: "pipeline".to_string(),
                            provider: None,
                            tool: None,
                            args: serde_json::Value::Null,
                            optional: false,
                        });
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
                    rule_refs: Vec::new(),
                    step_type: "pipeline".to_string(),
                    provider: None,
                    tool: None,
                    args: serde_json::Value::Null,
                    optional: false,
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
        // PROVIDER STEPS (2026-11-04) are exempt from this coercion: `step_type == "provider"` steps run an MCP provider,
        // not a pipeline (pipeline_id 0), see execute_provider_step.
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
            if !step.is_provider_step() && step.pipeline_id != 9 && step.pipeline_id != 56 {
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
            // MODEL_OVERRIDE WHITELIST (perimeter review 2026-10-23 — the
            // model-text→executable path, closed at the walk boundary): a
            // blueprint step's model_override is trusted ONLY for backend
            // selection (identifier/type/endpoint-agnostic sizing fields).
            // The executable/secret fields — bitnet_cli_path (pipeline 9
            // RUNS it), local_model_path, api_endpoint, api_key(_env) — are
            // model-authored text and are STRIPPED here: an injected or
            // hallucinated blueprint must never choose what binary runs or
            // where requests go. Loop counts are capped for the same reason
            // (max_iterations came straight from the model, unbounded).
            if let Some(mo) = step.model_override.as_mut() {
                if mo.bitnet_cli_path.is_some() || mo.local_model_path.is_some() || mo.api_endpoint.is_some() {
                    tracing::warn!(
                        step = step.step_index,
                        "SECURITY: blueprint model_override carried executable/endpoint fields (bitnet_cli_path/local_model_path/api_endpoint) — STRIPPED (walk-owned values apply)"
                    );
                }
                mo.bitnet_cli_path = None;
                mo.local_model_path = None;
                mo.api_endpoint = None;
                mo.api_key = None;
                mo.api_key_env = None;
            }
            if let Some(lc) = step.loop_config.as_mut() {
                const MAX_LOOP_ITERATIONS: u32 = 1_000;
                if lc.max_iterations > MAX_LOOP_ITERATIONS {
                    tracing::warn!(
                        step = step.step_index,
                        from = lc.max_iterations,
                        capped = MAX_LOOP_ITERATIONS,
                        "SECURITY: blueprint loop_config.max_iterations capped (model-authored, was unbounded)"
                    );
                    lc.max_iterations = MAX_LOOP_ITERATIONS;
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
                    rule_refs: Vec::new(),
                    step_type: "pipeline".to_string(),
                    provider: None,
                    tool: None,
                    args: serde_json::Value::Null,
                    optional: false,
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
                        rule_refs: Vec::new(),
                        step_type: "pipeline".to_string(),
                        provider: None,
                        tool: None,
                        args: serde_json::Value::Null,
                        optional: false,
                    });
                    super::amt::record_intent_event(
                        state,
                        "intent_carried",
                        serde_json::json!({ "status": "appended", "intent": request_text }),
                    );
                }
            }
        }

        // Store blueprint in ZSEI — as a TYPED Container whose content
        // (name, description, steps, missing_capabilities) lives in a body
        // file pointed at by storage.object_store_path: the codebase's one
        // content-pointer convention (same as bootstrap blueprints and
        // consciousness insights), and exactly where the reuse path and
        // load_prior_design_context read from. The previous flat JSON
        // ({container_type, metadata, context, storage:{steps}}) could not
        // deserialize into Container (no global_state/local_state), so
        // create_container always failed and the failure was swallowed.
        // PROVIDER STEPS (tool_binding.rs): validate model provider steps, insert the methodologies' declared per-rule providers as
        // provider steps before their carriers (authoritative), and CAPTURE every missing capability durably — not just log it.
        self.judge_rule_applicability(state).await;
        Self::audit_rule_refs(state);
        let all_missing = Self::reconcile_tool_bindings(state, &name, &missing_capabilities).await;
        let blueprint_body = serde_json::json!({
            "name": name,
            "description": description,
            "created_by": "orchestrator",
            "steps": state.blueprint_steps,
            "missing_capabilities": all_missing,
            "tool_manifest": state.tool_manifest,
            "methodology_ids": state.methodologies,
        });
        match self
            .persist_blueprint_container(&name, state, &blueprint_body)
            .await
        {
            Ok(new_id) => {
                state.blueprint_id = Some(new_id);
                state.blueprints_created += 1;
            }
            Err(e) => {
                tracing::warn!(error = %e, "stage 3: new blueprint could not be persisted — this walk proceeds, but nothing is reusable later");
            }
        }

        self.record_stage_timed(
            state,
            6,
            "Blueprint Assignment",
            true,
            &format!(
                "Created new blueprint with {} steps (missing: {})",
                state.blueprint_steps.len(),
                all_missing.len()
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
        self.emit_marker(state, "[7.0]", "");

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
                    match Self::rules_text_for(state, method_id, &container) {
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
                // RANKED, token-aware (2026-11-08): this was the substring scan over ALL keywords — the flood shape
                // (hundreds of hits fetched name-by-name into the simulation prompt). Best-first now: the same
                // names, the best matches first, the flood gone.
                .search_ranked(
                    &state.keywords.iter().cloned().collect::<Vec<_>>(), // ALL keywords (audit #15)
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
        // PROVIDER STEP CHECKS (2026-11-04) — HOST-COMPUTED, not model-guessed: for every provider step the registry says whether
        // its provider is present, degraded (health ledger) or missing. The old prompt asked the model to "flag any step whose goal
        // matches a registered capability" with NO slot to put the flag in; availability is now a fact the host establishes, the
        // model only judges the plan. A missing REQUIRED provider is captured as a missing capability (durable + ripple +
        // methodology gap) and the step will fail honestly at execution (pause, resumable after the capability is installed).
        let (provider_checks, provider_checks_block) = {
            use super::tool_binding::{degraded, providers_of, record_capability_gaps, GapInput, RegistryRow};
            let entries = crate::mcp::registry_entries().await.unwrap_or_default();
            let rows: Vec<RegistryRow> = entries.iter().map(|t| (t.name.clone(), t.capabilities.clone(), t.endpoint.clone())).collect();
            let providers = providers_of(&rows);
            let mut checks: Vec<serde_json::Value> = Vec::new();
            let mut gap_inputs: Vec<GapInput> = Vec::new();
            for st in state.blueprint_steps.iter().filter(|s| s.is_provider_step()) {
                let wanted = st.provider.clone().unwrap_or_default();
                let found = providers.keys().find(|k| k.eq_ignore_ascii_case(&wanted)).cloned();
                let status = match &found {
                    None => "missing",
                    Some(k) if degraded(k) => "degraded",
                    Some(_) => "ready",
                };
                if status == "missing" && !st.optional {
                    gap_inputs.push(GapInput {
                        kind: "provider".to_string(),
                        name: wanted.clone(),
                        source: "simulation".to_string(),
                        detail: format!("provider step {} cannot run — provider not registered", st.step_index),
                    });
                }
                checks.push(serde_json::json!({
                    "step": st.step_index, "provider": wanted, "status": status, "optional": st.optional,
                    "tools": found.as_ref().and_then(|k| providers.get(k)).cloned().unwrap_or_default(),
                    "rule_refs": st.rule_refs,
                }));
            }
            record_capability_gaps(&gap_inputs);
            for g in &gap_inputs {
                if !state.missing_capabilities.iter().any(|m| m["name"].as_str() == Some(g.name.as_str())) {
                    state.missing_capabilities.push(serde_json::json!({ "kind": g.kind, "name": g.name, "source": "simulation", "status": "missing" }));
                }
            }
            let block = if checks.is_empty() {
                "(no provider steps in this blueprint)".to_string()
            } else {
                checks
                    .iter()
                    .map(|c| format!("  - step {}: provider {} — {}{}", c["step"], c["provider"].as_str().unwrap_or(""), c["status"].as_str().unwrap_or(""), if c["optional"].as_bool().unwrap_or(false) { " (optional)" } else { "" }))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            (checks, block)
        };
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

REGISTERED CAPABILITIES (tools/MCPs callable via /mcp/call):
{capabilities}

PROVIDER STEP AVAILABILITY (computed by the host from the registry and the health ledger — facts, not predictions; a
step marked "missing" cannot run until its provider is installed, and that is already recorded as a missing capability):
{provider_checks_block}

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
                    let route = if s.is_provider_step() {
                        format!("provider {}", s.provider.as_deref().unwrap_or("?"))
                    } else {
                        format!("pipeline {}", s.pipeline_id)
                    };
                    format!(
                        "Step {}: {} - {} ({}){}",
                        s.step_index, s.action, s.description, route, ctx_req
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
        // No silent `{}`: an unreadable simulation is a FAILED stage, not "medium
        // feasibility, no predictions, no clarifications".
        let response = Self::response_text(&sim_result).ok_or_else(|| {
            "Zero-Shot Simulation: the model returned no response text — not substituting an empty simulation".to_string()
        })?;
        let sim_json = Self::parse_json_object_strict(response)
            .map_err(|e| format!("Zero-Shot Simulation: {e}"))?;

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
        if !provider_checks.is_empty() {
            // the host-computed checks are part of the simulation record (Raw Thoughts), not only a log line
            self.record_thinking(
                state,
                "Simulation — provider step checks",
                &serde_json::json!({ "response": provider_checks_block, "tool_checks": provider_checks }),
            );
        }

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
        self.emit_marker(state, "[8.1]", "");

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
                match Self::rules_text_for(state, method_id, &container) {
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
        // DOCTRINE (2026-11-01): the consciousness gate is decided ONLY by
        // meta/consciousness. If it cannot judge (error, rate limit, empty,
        // ReviewPending/ReviewFailed, no decision) the walk AWAITS in a
        // durable awaiting item and re-asks with backoff — it never fails
        // open and never hands the decision to a human. The orchestrate walk
        // is detached, so waiting does not hold a client connection.
        let mut await_id: Option<String> = None;
        let result = loop {
            self.emit_marker(state, "[8.2]", "");
            let attempt = self.executor.execute(39, input.clone()).await;
            tracing::info!(stage = 8, "[8.3] gate execute returned");
            let (judged, why) = match &attempt {
                Ok(v) => {
                    let d = v.get("gate").and_then(|g| g.get("decision")).and_then(|d| d.as_str());
                    match d {
                        None => (false, "gate returned no decision".to_string()),
                        Some("ReviewPending") | Some("ReviewFailed") => (
                            false,
                            v.get("gate")
                                .and_then(|g| g.get("reasoning"))
                                .and_then(|r| r.as_str())
                                .unwrap_or("meta could not judge (rate limit / empty / all models failed)")
                                .to_string(),
                        ),
                        Some(_) => (true, String::new()),
                    }
                }
                Err(e) => (false, format!("gate call failed: {}", e)),
            };
            if judged {
                if let Some(id) = &await_id {
                    super::awaiting::mark_resolved(&self.data_dir, id, "meta judged");
                    super::awaiting::mark_applied(&self.data_dir, id);
                }
                break attempt?;
            }
            let item = match &await_id {
                Some(id) => super::awaiting::record_attempt(&self.data_dir, id, &why),
                None => {
                    let subject: String = state.cleaned_prompt.clone(); // whole (no-cap doctrine)
                    let it = super::awaiting::open(
                        &self.data_dir,
                        super::awaiting::KIND_CONSCIOUSNESS_GATE,
                        &subject,
                        state.task_id.map(|t| t.to_string()).as_deref(),
                        None,
                        &why,
                    )
                    .ok();
                    if let Some(it) = &it {
                        await_id = Some(it.await_id.clone());
                    }
                    it
                }
            };
            let attempts = item.as_ref().map(|i| i.attempts).unwrap_or(0);
            tracing::warn!(
                stage = 8,
                attempts,
                "[8.3w] consciousness gate AWAITING meta judgment (not proceeding): {}",
                why
            );
            self.record_stage(
                state,
                8,
                "Consciousness Gate",
                true,
                &format!("AWAITING meta judgment (attempt {}): {}", attempts, why),
            );
            let wait = std::time::Duration::from_secs(super::awaiting::backoff_secs(attempts));
            let _ = tokio::time::timeout(wait, super::awaiting::await_wake().notified()).await;
            if let Some(id) = &await_id {
                if let Some(cur) = super::awaiting::get(&self.data_dir, id) {
                    if cur.status == super::awaiting::STATUS_CANCELLED {
                        return Err(format!("AWAITING:{} cancelled", id));
                    }
                }
            }
        };

        let decision = result
            .get("gate")
            .and_then(|g| g.get("decision"))
            .and_then(|d| d.as_str())
            .unwrap_or("Decline");

        let reasoning = result
            .get("gate")
            .and_then(|g| g.get("reasoning"))
            .and_then(|r| r.as_str())
            .unwrap_or("No reasoning provided")
            .to_string();

        state.gate_result = Some(GateResult {
            decision: decision.to_string(),
            reasoning: reasoning.clone(),
        });

        if decision == "Decline" {
            return Err(format!("Consciousness gate declined: {}", reasoning));
        }

        self.emit_marker(state, "[8.3]", &format!("decision: {}", decision));
        self.record_stage_timed(
            state,
            8,
            "Consciousness Gate",
            true,
            &format!("Decision: {} [gate-reported]", decision),
            stage_start.elapsed().as_millis() as u64,
        );

        Ok(())
    }

    /// Re-enter a failure-paused run: the step loop (which skips completed steps), then the
    /// same result collection / post-execution / delivery stages as a fresh run.
    pub(crate) async fn execute_resumed_stages(&self, state: &mut OrchestrationState) -> Result<(), String> {
        self.stage_6_to_8_execute_steps(state).await?;
        self.stage_9_result_collection(state).await?;
        if state.request.consciousness_enabled {
            self.stage_10_post_execution(state).await?;
        } else {
            self.record_stage(state, 12, "Post-execution", true, "Skipped (disabled)");
        }
        self.deliver_then_link(state).await
    }

    async fn stage_6_to_8_execute_steps(
        &self,
        state: &mut OrchestrationState,
    ) -> Result<(), String> {
        let stage_start = std::time::Instant::now();

        // STAGE 7: Create task via TaskManager (skipped on resume: the task already exists)
        if state.task_id.is_none() {
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
                // participants (src/walk_feed.rs): bind the walk's trace to its task AT CREATION, so the agents taking part
                // resolve the walk by task id for its whole life, not only once it returns
                if let Some(t) = crate::trace::current_trace() {
                    crate::walk_feed::bind_task(&t.trace_id, task_id);
                }

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
        }
        // WORK-ORDER TREE (operator 2026-11-10): record this walk's structure (AMT branches, steps, capability-gap checklists) on the task
        // records; idempotent, so a resumed walk only adds what is new.
        self.materialize_work_order(state).await;
        // Initial checkpoint: from here a failure can pause + resume this run.
        self.write_checkpoint(state, None);

        // STAGES 6 & 8: Execute steps
        // RESUME: steps whose output is already recorded (restored from the run checkpoint)
        // are complete and are not re-run; a fresh run starts with both sets empty.
        let mut completed: HashSet<u32> = state.step_outputs.keys().cloned().collect();
        let mut all_outputs: Vec<String> = state
            .step_results
            .iter()
            .filter_map(|r| {
                state
                    .step_outputs
                    .get(&r.step_index)
                    .and_then(|o| o.get("output"))
                    .and_then(|o| o.as_str())
                    .map(String::from)
            })
            .collect();

        // EXPANSION ROUNDS (operator 2026-11-11: the AMT grows while the task is open; the task completes only when ALL steps complete): the
        // step queue is built from the plan as it is NOW. When a step's alignment review discovered branches (`state.walk_expansion`) the inner
        // loop stops before its next step, `expand_walk` turns them into AMT nodes / closed gaps / appended steps / work-order items, and the
        // next round picks up the remaining + new steps. A round with nothing queued ends the loop, exactly as before.
        loop {
        let steps = state.blueprint_steps.clone();
        // Build dependency order
        let mut step_queue: Vec<&BlueprintStep> =
            steps.iter().filter(|s| !completed.contains(&s.step_index)).collect();
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
            // a step discovered new AMT branches: expand them (below) before the next step starts
            if !state.walk_expansion.is_empty() {
                break;
            }
            // the next step fans out lane subprocesses: pause (never skip, never fail) until the host clears the memory floor
            crate::wait_for_memory_floor("walk step").await;

            // Find steps whose dependencies are satisfied
            let ready_steps: Vec<_> = step_queue
                .iter()
                .filter(|s| s.depends_on.iter().all(|dep| completed.contains(dep)))
                .cloned()
                .collect();

            if ready_steps.is_empty() && !step_queue.is_empty() {
                // Force execute first remaining step (break deadlock)
                if let Some(step) = step_queue.first().cloned() {
                    // FAILURE DOCTRINE: never continue past a failed step — pause the task here.
                    let result = match self.execute_step(state, step, &all_outputs).await {
                        Ok(r) => r,
                        Err(e) => return Err(self.pause_on_step_error(&*state, step.step_index, &e).await),
                    };
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
                    self.write_checkpoint(state, None);

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
                        // the step ran to completion: tick its work-order item (completion cascades by the one rule)
                        if let Some(pos) = steps.iter().position(|s| std::ptr::eq(s, step)) {
                            self.work_order_step_done(state, pos, &format!("step executed to completion ({} tokens)", result.tokens_used)).await;
                        }
                    }
                }
            } else {
                for step in ready_steps {
                    // FAILURE DOCTRINE: never continue past a failed step — pause the task here.
                    let result = match self.execute_step(state, step, &all_outputs).await {
                        Ok(r) => r,
                        Err(e) => return Err(self.pause_on_step_error(&*state, step.step_index, &e).await),
                    };
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
                    self.write_checkpoint(state, None);

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
                        // the step ran to completion: tick its work-order item (completion cascades by the one rule)
                        if let Some(pos) = steps.iter().position(|s| std::ptr::eq(s, step)) {
                            self.work_order_step_done(state, pos, &format!("step executed to completion ({} tokens)", result.tokens_used)).await;
                        }
                    }
                }
            }
        }

        // the inner loop ended: cancelled → stop; nothing discovered → the plan is complete; else EXPAND and run another round
        if let Some(task_id) = state.task_id {
            if self.task_manager.read().await.is_cancelled(task_id).await {
                break;
            }
        }
        if state.walk_expansion.is_empty() {
            break;
        }
        let discovered_at = state.walk_expansion.first().and_then(|p| p["step"].as_u64()).unwrap_or(0) as u32;
        let added_steps = match self.expand_walk(state).await {
            Ok(n) => n,
            Err(e) => {
                // FAILURE DOCTRINE: an expansion that cannot be completed pauses the task (queue + checkpoint kept); it is never skipped
                return Err(self.pause_on_step_error(&*state, discovered_at, &format!("AMT expansion while the task was open: {e}")).await);
            }
        };
        // The plan now holds steps the stage-4 simulation never saw: simulate it again (operator 2026-11-11). The result REPLACES the earlier
        // simulation on the state (it describes the plan as it is now); it informs, it does not re-open decisions already taken. A failed
        // simulation pauses the task like every other failure — never skipped, never continued past.
        if added_steps > 0 {
            if let Err(e) = self.stage_4_zero_shot_simulation(state).await {
                return Err(self.pause_on_step_error(&*state, discovered_at, &format!("re-simulation after AMT expansion: {e}")).await);
            }
            let feasibility = state.simulation_result.as_ref().map(|r| r.overall_feasibility.clone());
            super::amt::record_intent_event(state, "walk_expansion_resimulated", serde_json::json!({"new_steps": added_steps, "feasibility": feasibility}));
        }
        self.write_checkpoint(state, None);
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

    /// PROVIDER STEP (2026-11-04; `step_type == "provider"` — operator: "provider is just a step type", a different step type
    /// beside pipeline steps). Calls the step's PROVIDER (an MCP server) through the gated, metered, jurisdiction-checked MCP path
    /// (tool_routing::call_provider_for_step): which of the provider's tools runs is decided at call time from the step's own
    /// text (or the pinned `tool`); the step text goes WHOLE unless the step carries structured `args`. Every produced
    /// container records where it was called from (provider, tool, site, task, step, rule — mcp.rs post-call hook →
    /// mcp_graph::CalledFrom). Output `{response, success, provider, tools, rule_refs}` flows to the carrying step as a
    /// previous-step output. A required provider that cannot be attempted or fails PAUSES the walk (failure doctrine, resumable);
    /// an OPTIONAL provider step that fails is recorded and skipped (never fabricated).
    async fn execute_provider_step(
        &self,
        state: &mut OrchestrationState,
        step: &BlueprintStep,
    ) -> Result<serde_json::Value, String> {
        let provider = step
            .provider
            .clone()
            .ok_or_else(|| format!("provider step {} names no provider", step.step_index))?;
        let mut attempt = super::tool_routing::call_provider_for_step(
            &step.description,
            &provider,
            step.tool.as_deref(),
            &step.args,
            &step.rule_refs,
            step.step_index,
            state.task_id,
        )
        .await;
        // TOOL CHOICE BY INTELLIGENCE (E17, 2026-11-07): when word-overlap cannot pick among a multi-tool provider's tools
        // ("none matches this step's text" — task 1148 step 0), ask the model WHICH tool fits instead of failing the step; only an
        // answer naming one of the provider's real tools is used (NONE or anything else keeps the honest failure).
        if let Err(e) = &attempt {
            if step.tool.is_none() && e.contains("none matches this step's text") {
                let entries = crate::mcp::registry_entries().await.unwrap_or_default();
                let tools: Vec<(String, String)> = entries
                    .iter()
                    .filter(|t| crate::mcp_supervisor::provider_for_endpoint(&t.endpoint).eq_ignore_ascii_case(&provider))
                    .map(|t| (t.name.clone(), t.capabilities.join(" ")))
                    .collect();
                let list = tools.iter().map(|(n, c)| format!("- {n}: {c}")).collect::<Vec<_>>().join("\n");
                let prompt = format!(
                    "STEP TO CARRY OUT:\n{}\n\nTHE PROVIDER \"{provider}\" OFFERS THESE TOOLS:\n{list}\n\nWhich ONE tool best carries out this step? Reply with ONLY the exact tool name from the list, or NONE if no tool fits the step.",
                    step.description
                );
                let input = serde_json::json!({
                    "prompt": prompt, "max_tokens": 40, "temperature": 0.0,
                    "system_prompt": "Reply with a single tool name from the list, or NONE. No other text."
                });
                if let Ok(res) = self.metered_execute_resilient(state, input, "provider_tool_choice").await {
                    let reply = self.extract_output_text(&res);
                    let picked = tools.iter().find(|(n, _)| reply.trim().trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-').eq_ignore_ascii_case(n)).map(|(n, _)| n.clone());
                    if let Some(tool) = picked {
                        tracing::info!(step = step.step_index, provider = %provider, tool = %tool, "provider tool chosen by the model (word overlap matched nothing)");
                        attempt = super::tool_routing::call_provider_for_step(
                            &step.description, &provider, Some(&tool), &step.args, &step.rule_refs, step.step_index, state.task_id,
                        )
                        .await;
                    }
                }
            }
        }
        let outcomes = match attempt {
            Ok(o) => o,
            Err(e) if step.optional => {
                return Ok(serde_json::json!({
                    "response": format!("[optional provider {provider} was not used: {e}]"),
                    "success": true, "skipped": true, "provider": provider, "rule_refs": step.rule_refs,
                }));
            }
            Err(e) => return Err(format!("provider step {} ({}) could not run: {}", step.step_index, provider, e)),
        };
        let any_ok = outcomes.iter().any(|o| o.success);
        if !any_ok {
            let why = outcomes.iter().map(|o| format!("{}: {}", o.tool, o.error.clone().unwrap_or_else(|| "failed".into()))).collect::<Vec<_>>().join("; ");
            if step.optional {
                return Ok(serde_json::json!({
                    "response": format!("[optional provider {provider} failed: {why}]"),
                    "success": true, "skipped": true, "provider": provider, "rule_refs": step.rule_refs,
                }));
            }
            return Err(format!("provider step {} ({}) failed: {}", step.step_index, provider, why));
        }
        Ok(serde_json::json!({
            "response": outcomes.iter().map(|o| o.text.clone()).collect::<Vec<_>>().join("\n"),
            "success": true,
            "provider": provider,
            "tools": outcomes.iter().map(|o| o.tool.clone()).collect::<Vec<_>>(),
            "rule_refs": step.rule_refs,
        }))
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
                let response = result.get("response").and_then(|r| r.as_str()).unwrap_or("");
                let json_str = Self::extract_json_from_response(response, '{', '}');
                serde_json::from_str::<serde_json::Value>(json_str.trim())
                    .ok()
                    .and_then(|v| v.get("sub_queries").cloned())
                    .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok())
                    .filter(|v| !v.is_empty())
                    .unwrap_or_else(|| {
                        // Designed degrade (search the original query), but never silent.
                        tracing::warn!(site = "web_search_decompose", "decomposition response had no usable sub_queries — searching the original query only");
                        vec![query.clone()]
                    })
            }
            Err(e) => {
                tracing::warn!(site = "web_search_decompose", error = %e, "decomposition call failed — searching the original query only");
                vec![query.clone()]
            }
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
        let step_note = format!("step {} (pipeline {})", step.step_index, step.pipeline_id);
        self.emit_marker(state, "[10.step]", &step_note);
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
            // PRIOR DESIGN CONTEXT (cross-walk design mechanism 1: inject the
            // matched prior blueprint's step content into the step's context —
            // the same design, already in the graph from a prior walk). The
            // store search is local + fast; the content is the actual step
            // text from a prior walk's design. Without this, the model
            // executing the step only sees THIS walk's session replay and
            // cannot build on prior work.
            let prior_design = self.load_prior_design_context(state, &step.description).await;
            // SPAN-OUT (2026-11-02): fan out from everything the run already captured
            // (AMT, prompt/chunk/file/modality graphs, blueprint, methodologies) along the
            // captured relationships until the window share is full. This replaces
            // "4 seeds x depth 2 x 8 containers" as the primary graph context; the
            // pipeline's keyword path still runs beside it.
            let span = self.span_out_for_step(state, step).await;
            let span_out_context: Option<String> = if span.text.trim().is_empty() { None } else { Some(span.text.clone()) };
            let context_input = serde_json::json!({
                "span_out_context": span_out_context,
                "action": "ForStep",
                "query": format!("{} - {}", state.cleaned_prompt, step.description),
                "session_context": session_context,
                "prior_design_context": prior_design,
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
                        {
                            let mut m = vec!["keyword-scan".to_string()];
                            if span.items.is_empty() {
                                m.push("span-out:empty".to_string());
                            } else {
                                m.push(format!(
                                    "span-out:{} items, {} tokens, {} fetched{}{}",
                                    span.items.len(),
                                    span.tokens,
                                    span.fetched,
                                    if span.budget_filled { ", budget filled" } else { "" },
                                    if span.fetch_cap_hit { ", FETCH CAP HIT" } else { "" },
                                ));
                            }
                            m
                        },
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
                // CHAT MEMORY (2026-11-02): the last turns of THIS chat scope (full
                // text) so a follow-up prompt sees the conversation, not only the
                // graph. Labelled as history, never as the task.
                if let Some(history) = self.recent_chat_context(state) {
                    fc.push_str("[Conversation so far in this chat (earlier turns, oldest first)]\n");
                    fc.push_str(&history);
                }
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
                // FAILURE DOCTRINE: a failed sub-step fails the step (which pauses the task);
                // it is never recorded and walked past.
                if let Err(e) = &sub_result {
                    return Err(format!(
                        "sub-step {} (pipeline {}) of step {} failed: {}",
                        sub_step.sub_index, sub_step.pipeline_id, step.step_index, e
                    ));
                }
            }

            // Execute main step — pipeline 56 (WebSearch) gets its own
            // input shape and its own AMT-style decomposition instead of
            // the generic pipeline-9 payload below (see the coercion
            // exception comment above and execute_web_search_step's own
            // doc comment for why decomposition lives here, not in the
            // pipeline itself).
            tracing::info!(stage = 10, "[10.i{}] step {} (pipeline {}) iteration begin", iteration, step.step_index, step.pipeline_id);
            if step.is_provider_step() {
                tracing::info!(stage = 10, "[10.p{}] provider step begin ({})", step.step_index, step.provider.as_deref().unwrap_or("?"));
                final_output = self.execute_provider_step(state, step).await?;
                tracing::info!(stage = 10, "[10.p{}] provider step done", step.step_index);
                self.record_thinking(
                    state,
                    &format!("Step Execution (step {})", step.step_index),
                    &final_output,
                );
                total_iterations = iteration + 1;
                break;
            }
            if step.pipeline_id == 56 {
                tracing::info!(stage = 10, "[10.w{}] web-search step begin", step.step_index);
                final_output = self.execute_web_search_step(state, step).await?;
                if final_output.get("success").and_then(|v| v.as_bool()) == Some(false) {
                    // Every sub-query failed: that is a failed step, not an empty search result.
                    return Err(format!(
                        "web search step {} failed: {}",
                        step.step_index,
                        final_output.get("error").and_then(|e| e.as_str()).unwrap_or("no sub-query succeeded")
                    ));
                }
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

            // TOOL ROUTING (directive #46 wired, 2026-10-23): with
            // OZONE_TOOL_ROUTING=1, simulation-selected tools may contribute
            // budget-capped, untrusted-labeled REAL output to the step's
            // prompt. Gate off (default) = byte-for-byte today's prompt.
            let step_prompt = match crate::orchestrator::tool_routing::tool_context_for_step_with_task(
                &step.description,
                &state.applicable_tools,
                state.task_id,
                &step.methodology_ids,
            )
            .await
            {
                // no fixed cut (no-cap doctrine): the block is fitted to the step model's window; any trim is recorded
                Some(block) => {
                    let block = super::amt::render_within_window(state, "tool_results", 6, block);
                    format!("{step_prompt}\n{block}")
                }
                None => step_prompt,
            };
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
        let compliance_check = if step.methodology_ids.is_empty() || step.is_provider_step() {
            None
        } else {
            let mut rules_text_parts = Vec::new();
            for &method_id in &step.methodology_ids {
                if let Ok(Some(container)) = self.store.get_container(method_id).await {
                    if let Some(rules) = Self::rules_text_for(state, method_id, &container) {
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

        // NO FAKE SUCCESS (2026-11-04, found by the clock E2E): a model that "uses a tool" by writing
        // `<tool_call>…` markup into its answer has executed NOTHING — the host never executes model-written
        // tool markup (tools run only through host tool routing). Task 994 "completed" with a narrated plan and
        // no clock files. Such a step is a failure to PAUSE (resumable), never a success.
        {
            let text = self.extract_output_text(&final_output);
            if let Some(marker) = unexecuted_tool_markup(&text) {
                return Err(format!(
                    "step {} produced tool-call markup ({}) that nothing executes — the model described tool use instead of the host routing it (tools run only via host tool routing: OZONE_TOOL_ROUTING=1 and a registered tool for this step). The step output was NOT accepted.",
                    step.step_index, marker
                ));
            }
        }

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

                // every step ran: tick the gap `verify` items first (they say "the walk ran all its steps"), then take the COMPLETION GATE
                // (operator 2026-11-11: a task completes only when ALL its steps and child work are complete; completed is final). Open child
                // work — an AMT branch/step item, an AMT re-expansion child task, a failed child not superseded — PARKS the task as
                // `awaiting_children` with its outputs kept; the last child's completion releases it (`TaskManager::release_awaiting`). It is
                // never completed early and never failed because of a child that is merely still running.
                self.work_order_walk_done(state).await;
                let gate = self
                    .task_manager
                    .read()
                    .await
                    .complete_or_park(
                        task_id,
                        Some(serde_json::json!({
                            "response": state.final_response,
                            "steps": outputs
                        })),
                        state.tokens_used_so_far,
                    )
                    .await;
                match gate {
                    Ok(true) => {}
                    Ok(false) => {
                        tracing::info!(task_id, "task parked as awaiting_children: its response is delivered, completion waits for open child work");
                        crate::orchestrator::amt::record_intent_event(
                            state,
                            "walk_awaiting_children",
                            serde_json::json!({"task_id": task_id, "note": "all steps ran; child work is still open — the task completes (final) when it does"}),
                        );
                    }
                    Err(e) => tracing::error!(task_id, error = %e, "completion gate: the task record could not be completed or parked"),
                }
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
                // FAILURE DOCTRINE: a run that produced no final response is not "done" — pause
                // it at the first step with no recorded output so it can be resumed from there.
                // Only when every step has output yet there is still no response is the task
                // marked failed (nothing to resume into).
                let first_missing = state
                    .blueprint_steps
                    .iter()
                    .find(|st| !state.step_outputs.contains_key(&st.step_index))
                    .map(|st| st.step_index);
                match first_missing {
                    Some(i) => {
                        let _ = self.pause_on_step_error(&*state, i, &message).await;
                    }
                    None => {
                        let _ = self
                            .task_manager
                            .read()
                            .await
                            .fail_task(task_id, message)
                            .await;
                    }
                }
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

        // EXPERIENCE + EMOTION (2026-11-02): pipelines 41 and 43 have no built binary
        // (every stage-12 run recorded "binary not built"), and the host store
        // already does this work: TaskManager's consciousness observer calls
        // `post_task_experience_with` when the task completes or fails (stage 11),
        // storing the experience AND moving the emotional state. Stage 12 therefore
        // VERIFIES that record in the host store instead of dispatching a duplicate.
        let experience_result: Result<serde_json::Value, String> = match state.task_id {
            Some(tid) => {
                let n = crate::consciousness::store::experience_count_for_task(tid);
                if n > 0 {
                    Ok(serde_json::json!({ "recorded_by": "task-manager-observer", "experiences": n }))
                } else if !crate::consciousness::store::is_consciousness_enabled() {
                    Err("skipped: consciousness disabled in the store".to_string())
                } else {
                    Err("no experience recorded for this task by the host store (observer did not run or experience_memory_enabled = false)".to_string())
                }
            }
            None => Err("no task id: nothing to verify".to_string()),
        };

        // Update relationship if we know the user
        // Conforms to pipeline 47's contract (`RecordInteraction { user_id, summary: InteractionSummary }`; the old flat fields made
        // every Stage 12 relationship update fail to parse — live walk 1155). Only REAL values are sent: the outcome, the topics, a
        // timestamp; no quality score or tone is invented (absent fields default in the pipeline).
        let relationship_input = serde_json::json!({
            "action": "RecordInteraction",
            "user_id": state.request.user_id,
            "summary": {
                "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
                "topic": state.topics.join(", "),
                "task_types": ["task_completion"],
                "outcome": if state.final_response.is_some() { "positive" } else { "negative" },
            }
        });

        // Pipeline 47 = RelationshipDevelopment
        // Honors [consciousness].relationship_system_enabled (was stored but unread).
        let relationship_result = if crate::consciousness::store::relationship_system_enabled() {
            self.executor.execute(47, relationship_input).await
        } else {
            Err("skipped: relationship_system_enabled = false".to_string())
        };

        // Emotional state: read the host store's live state (moved by the same observer).
        let emotion_result: Result<serde_json::Value, String> = {
            let e = crate::consciousness::store::get_current_emotional_state();
            Ok(serde_json::json!({ "primary": e.primary_emotion, "valence": e.valence, "arousal": e.arousal }))
        };

        // HONEST stage record: which of the three really ran, which failed
        // (with the real error) — never a blanket success over dropped calls.
        let outcomes = [
            (41u64, "experience", &experience_result),
            (47u64, "relationship", &relationship_result),
            (43u64, "emotion", &emotion_result),
        ];
        let mut ok: Vec<String> = Vec::new();
        let mut failed: Vec<String> = Vec::new();
        for (pipeline, label, result) in &outcomes {
            match result {
                Ok(_) => ok.push(label.to_string()),
                // A config-disabled subsystem is a deliberate skip, not a failure.
                Err(e) if e.starts_with("skipped:") => ok.push(format!("{label}(skipped by config)")),
                // An unbuilt pipeline binary is "unavailable", reported as such (not hidden, not a run failure).
                Err(e) if e.contains("binary not built") => {
                    failed.push(format!("{label}({pipeline}): unavailable — {e}"));
                }
                Err(e) => {
                    failed.push(format!("{label}({pipeline}): {e}"));
                    tracing::warn!(pipeline, stage = 12, error = %e, "post-execution consciousness call FAILED (recorded, not dropped)");
                }
            }
        }
        let detail = if failed.is_empty() {
            format!("experience/relationship/emotion pipelines ok ({})", ok.join(", "))
        } else if ok.is_empty() {
            format!("ALL post-execution consciousness calls FAILED: {}", failed.join("; "))
        } else {
            format!("partial: ok=[{}] failed=[{}]", ok.join(", "), failed.join("; "))
        };
        self.record_stage_timed(
            state,
            12,
            "Post-execution Consciousness",
            failed.is_empty(), // truth: success only when NO call failed (was inverted)
            &detail,
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

/// Model-written tool-call markup that the host never executes. Returns the matched marker.
/// Deliberately requires an explicit tag form (a prose mention of the words "tool call" never matches).
pub(crate) fn unexecuted_tool_markup(text: &str) -> Option<&'static str> {
    const MARKERS: [&str; 6] = [
        "<tool_call>", "</tool_call>", "<function_calls>", "<invoke name=", "<param_key>", "<|tool_call",
    ];
    MARKERS.iter().copied().find(|m| text.contains(m))
}

#[cfg(test)]
mod tool_markup_tests {
    use super::unexecuted_tool_markup;
    #[test]
    fn detects_model_written_tool_markup_but_not_prose() {
        assert_eq!(unexecuted_tool_markup("I'll check.<tool_call>bash<param_key>command</param_key></tool_call>"), Some("<tool_call>"));
        assert_eq!(unexecuted_tool_markup("x <invoke name=\"bash\"> y"), Some("<invoke name="));
        assert!(unexecuted_tool_markup("The tool call finished and the function call returned 3.").is_none());
        assert!(unexecuted_tool_markup("").is_none());
    }
}
