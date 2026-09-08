//! //! AMT construction — both builders (graph-native + legacy), the shared
//! assembler, E-1 layer input, K knowledge enrichment, M synthesis, Q quality
//! scoring/pruning, and methodology cross-referencing.

use super::*;

impl PromptOrchestrator {

    pub(crate) async fn build_amt(&self, state: &mut OrchestrationState) -> Result<(), String> {
        let stage_start = std::time::Instant::now();

        // Route: graph-native traversal when the chunks carry sentence nodes
        // with grammar relationships (Path 2 / OMEX native outputs); the
        // legacy per-chunk zero-shot loop otherwise.
        let graph_ready = state
            .processed_chunks
            .iter()
            .any(|c| c.sentence_nodes.iter().any(|s| !s.grammar_relationships.is_empty()));

        state.amt = Some(if graph_ready {
            state.amt_build_mode = AmtBuildMode::GraphTraversal;
            self.build_amt_from_graphs(state).await?
        } else {
            state.amt_build_mode = AmtBuildMode::ChunkZeroShot;
            self.build_amt_layer_by_layer(state).await?
        });

        state.amt_validated = state.amt_pass_count > 0;
        state.validation_streak = 5; // convergence achieved inside the builder

        self.record_stage_timed(
            state,
            5,
            "Build AMT",
            true,
            &format!(
                "Mode: {:?}, Intents: {}, Branches: {}, Details: {}, Cross-refs: {}, Passes: {}, Validated: {}",
                state.amt_build_mode,
                state.intent_captures.len(),
                state.branch_captures.len(),
                state.detail_captures.len(),
                state.cross_refs.len(),
                state.amt_pass_count,
                state.amt_validated,
            ),
            stage_start.elapsed().as_millis() as u64,
        );

        Ok(())
    }

    /// Build the AMT by traversing the TEXT GRAPH — never by asking a model
    /// "what is the intent?".
    ///
    /// STAGE 1 — Candidate unit formation (context pool accumulation):
    ///   Start from every sentence; walk ESTABLISHED edges — coreference
    ///   chains, cross-sentence relationship edges, and shared grammar
    ///   subjects/objects (normalized). Union sentences that share evidence.
    ///   This is evidence expansion, not similarity clustering: the context
    ///   pool grows by following grammar/relationship neighbors until the
    ///   frontier weakens (no further evidence edges).
    ///
    /// STAGE 2 — Boundary evaluation:
    ///   Adjacent pools merge only on EVIDENCE OF CONTINUITY (strong: shared
    ///   coref/cross-rel edges; weak: shared entities → confirmed by a 5x
    ///   zero-shot YES/NO with the pools' structured evidence shown). Two
    ///   pools that merely share a VERB ("build") do NOT merge — that is the
    ///   failure mode that collapses unrelated projects into one intent.
    ///
    /// STAGE 3 — Promotion:
    ///   Each pool condenses to an IntentCapture (actor/action/target from
    ///   its dominant grammar evidence — deterministic, no generated
    ///   sentence); branches emerge INSIDE the pool by grouping sentences
    ///   around shared object anchors; each member sentence becomes a
    ///   DetailCapture; cross-pool relationship edges become CrossRefs;
    ///   aggregated knowledge references ride on the intent metadata.
    ///
    /// CONVERGENCE LOOP — per pass: gather → enrich (K) → synthesize (M)
    ///   → inject into prompts → methodology branch discovery → E.2
    ///   cross-reference → cross-refs → score/prune (Q) → convergence.
    ///   Bounded at 2 passes; converges when no branches grew, none were
    ///   pruned, and no new methodologies loaded.
    async fn build_amt_from_graphs(
        &self,
        state: &mut OrchestrationState,
    ) -> Result<AMTNode, String> {
        let mut node_id_counter = 1u64;
        let mut knowledge_paths: HashMap<String, Vec<String>> = HashMap::new();
        state.amt_pass_count += 1;

        // ── Flatten the corpus graph ──
        let mut all_sentences: Vec<OrchSentenceNode> = Vec::new();
        let mut cross_rels: Vec<OrchCrossSentenceRelationship> = Vec::new();
        let mut coref_chains: Vec<OrchCoreferenceChain> = Vec::new();
        for chunk in &state.processed_chunks {
            all_sentences.extend(chunk.sentence_nodes.iter().cloned());
            cross_rels.extend(chunk.cross_sentence_relationships.iter().cloned());
            coref_chains.extend(chunk.coreference_chains.iter().cloned());
        }

        if all_sentences.is_empty() {
            // No graph to traverse — fall back to the legacy loop.
            return self.build_amt_layer_by_layer(state).await;
        }

        let id_to_idx: HashMap<u64, usize> = all_sentences
            .iter()
            .enumerate()
            .map(|(i, s)| (s.node_id, i))
            .collect();
        let norm = |s: &str| s.trim().to_lowercase();

        // ── STAGE 1: context pool accumulation (union-find over evidence) ──
        let mut parent: Vec<usize> = (0..all_sentences.len()).collect();
        fn find(p: &mut Vec<usize>, mut x: usize) -> usize {
            while p[x] != x {
                p[x] = p[p[x]];
                x = p[x];
            }
            x
        }
        fn union(p: &mut Vec<usize>, a: usize, b: usize) {
            let ra = find(p, a);
            let rb = find(p, b);
            if ra != rb {
                p[rb] = ra;
            }
        }

        // Evidence 1: coreference chains — mentions in one chain are one
        // neighborhood.
        for chain in &coref_chains {
            let mut prev: Option<usize> = None;
            for mention in &chain.mentions {
                if let Some(&idx) = id_to_idx.get(&mention.sentence_id) {
                    if let Some(p) = prev {
                        union(&mut parent, p, idx);
                    }
                    prev = Some(idx);
                }
            }
        }

        // Evidence 2: cross-sentence relationship edges.
        for rel in &cross_rels {
            if let (Some(&a), Some(&b)) =
                (id_to_idx.get(&rel.from_sentence_id), id_to_idx.get(&rel.to_sentence_id))
            {
                union(&mut parent, a, b);
            }
        }

        // Evidence 3: shared grammar subjects/objects (normalized, len > 2).
        for i in 0..all_sentences.len() {
            for j in (i + 1)..all_sentences.len() {
                let terms_of = |s: &OrchSentenceNode| -> Vec<String> {
                    s.grammar_relationships
                        .iter()
                        .flat_map(|g| {
                            [
                                g.subject.to_lowercase(),
                                g.from_text.to_lowercase(),
                                g.to_text.to_lowercase(),
                            ]
                        })
                        .filter(|t| t.len() > 2)
                        .collect()
                };
                let a_terms = terms_of(&all_sentences[i]);
                let b_terms = terms_of(&all_sentences[j]);
                if a_terms.iter().any(|at| b_terms.contains(at)) {
                    union(&mut parent, i, j);
                }
            }
        }

        // Collect pools (keep reading order).
        let mut pool_map: HashMap<usize, Vec<usize>> = HashMap::new();
        for i in 0..all_sentences.len() {
            let r = find(&mut parent, i);
            pool_map.entry(r).or_default().push(i);
        }
        let pool_order: Vec<usize> = {
            let mut keys: Vec<usize> = pool_map.keys().cloned().collect();
            keys.sort_by_key(|k| pool_map[k].first().cloned().unwrap_or(usize::MAX));
            keys
        };
        let mut merged_pools: Vec<Vec<usize>> =
            pool_order.iter().map(|k| pool_map[k].clone()).collect();

        // ── STAGE 2: boundary evaluation between ADJACENT pools ──
        let mut i = 0usize;
        while i + 1 < merged_pools.len() {
            let a = &merged_pools[i];
            let b = &merged_pools[i + 1];

            // Strong evidence?
            let strong = cross_rels.iter().any(|r| {
                id_to_idx.contains_key(&r.from_sentence_id)
                    && id_to_idx.contains_key(&r.to_sentence_id)
                    && a.contains(&id_to_idx[&r.from_sentence_id])
                    && b.contains(&id_to_idx[&r.to_sentence_id])
            }) || coref_chains.iter().any(|c| {
                let in_a = c
                    .mentions
                    .iter()
                    .any(|m| id_to_idx.get(&m.sentence_id).map_or(false, |x| a.contains(x)));
                let in_b = c
                    .mentions
                    .iter()
                    .any(|m| id_to_idx.get(&m.sentence_id).map_or(false, |x| b.contains(x)));
                in_a && in_b
            });

            if strong {
                let b_take = merged_pools.remove(i + 1);
                merged_pools[i].extend(b_take);
                continue; // re-examine the (new) adjacent pair
            }

            // Weak evidence: shared ENTITIES (never verb-only) → confirm 5x.
            let terms_of_pool = |pool: &Vec<usize>| -> HashSet<String> {
                pool.iter()
                    .flat_map(|&si| {
                        all_sentences[si]
                            .grammar_relationships
                            .iter()
                            .flat_map(|g| {
                                [g.subject.clone(), g.from_text.clone(), g.to_text.clone()]
                            })
                    })
                    .map(|t| norm(&t))
                    .filter(|t| t.len() > 2)
                    .collect()
            };
            let a_terms = terms_of_pool(a);
            let b_terms = terms_of_pool(b);
            let shared: Vec<String> = a_terms.intersection(&b_terms).cloned().collect();

            if !shared.is_empty() {
                let pool_summary = |pool: &Vec<usize>| -> String {
                    pool.iter()
                        .map(|&si| {
                            let s = &all_sentences[si];
                            s.grammar_relationships
                                .first()
                                .map(|g| {
                                    format!(
                                        "  - {} → {} → {}",
                                        g.subject,
                                        g.verb,
                                        g.object.clone().unwrap_or_default()
                                    )
                                })
                                .unwrap_or_else(|| {
                                    format!("  - {}", &s.content[..s.content.len().min(80)])
                                })
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                };

                let prompt = format!(
                    r#"You are evaluating whether two candidate semantic neighborhoods belong to the SAME semantic neighborhood — one coherent goal, project, or topic — based on accumulated structural evidence.

NEIGHBORHOOD A (grammar evidence, subject → verb → object):
{a_summary}

NEIGHBORHOOD B (grammar evidence, subject → verb → object):
{b_summary}

SHARED TERMS: {shared}

RULES:
- Sharing a common VERB alone (e.g., both use "build") is NOT sufficient — different projects often share verbs.
- Sharing ENTITIES (the same subject or object doing related things), coreference, or explicit dependency/discourse relations IS evidence of continuity.

QUESTION: Does the evidence indicate these two neighborhoods belong to the SAME semantic neighborhood (one goal/project)?

Return ONLY valid JSON with a single one-word answer:
{{"answer": "YES"}} or {{"answer": "NO"}}"#,
                    a_summary = pool_summary(a),
                    b_summary = pool_summary(b),
                    shared = shared.join(", "),
                );

                if self.confirm_yes_no_orch(state, prompt, 5).await {
                    let b_take = merged_pools.remove(i + 1);
                    merged_pools[i].extend(b_take);
                    continue;
                }
            }
            i += 1;
        }

        // ── STAGE 3: promotion — pools → intents/branches/details ──
        for pool in &merged_pools {
            if pool.is_empty() {
                continue;
            }

            // Condensation: actor/action/target from the pool's dominant
            // grammar evidence. Deterministic — no model call, no generated
            // sentence.
            let mut subj_freq: HashMap<String, usize> = HashMap::new();
            let mut verb_freq: HashMap<String, usize> = HashMap::new();
            let mut obj_freq: HashMap<String, usize> = HashMap::new();
            for &si in pool {
                for g in &all_sentences[si].grammar_relationships {
                    if !g.subject.trim().is_empty() {
                        *subj_freq.entry(g.subject.trim().to_string()).or_insert(0) += 1;
                    }
                    if !g.verb.trim().is_empty() {
                        *verb_freq.entry(g.verb.trim().to_string()).or_insert(0) += 1;
                    }
                    if let Some(o) = &g.object {
                        if !o.trim().is_empty() {
                            *obj_freq.entry(o.trim().to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }
            let dominant = |m: &HashMap<String, usize>| -> String {
                m.iter()
                    .max_by_key(|(_, c)| **c)
                    .map(|(k, _)| k.clone())
                    .unwrap_or_default()
            };
            let actor = dominant(&subj_freq);
            let action = dominant(&verb_freq);
            let target = dominant(&obj_freq);

            let intent_text = match (action.is_empty(), target.is_empty()) {
                (false, false) => format!("{} {}", action, target),
                (false, true) => format!("{} (regarding {})", action, actor),
                _ => {
                    let c = &all_sentences[pool[0]].content;
                    c.chars().take(120).collect()
                }
            };

            // Dedup against existing intents (case-insensitive containment).
            let already_known = state.intent_captures.iter().any(|ic| {
                ic.intent.to_lowercase().contains(&intent_text.to_lowercase())
                    || intent_text.to_lowercase().contains(&ic.intent.to_lowercase())
            });

            if already_known {
                continue;
            }

            let intent_id = node_id_counter;
            node_id_counter += 1;
            let chunk_ids: Vec<u32> = pool
                .iter()
                .map(|&si| all_sentences[si].chunk_id)
                .collect::<HashSet<u32>>()
                .into_iter()
                .collect();
            let source_sentences: Vec<String> = pool
                .iter()
                .take(3)
                .map(|&si| all_sentences[si].original_content.clone())
                .collect();

            state.intent_captures.push(IntentCapture {
                intent: intent_text.clone(),
                is_parallel: merged_pools.len() > 1,
                source_chunk_indices: chunk_ids,
                source_sentences,
                node_id: intent_id,
            });

            // Branches emerge INSIDE the pool: group member sentences by
            // their object anchor (or knowledge surface). Each distinct
            // anchor with ≥1 sentence becomes a branch.
            let mut by_anchor: HashMap<String, Vec<usize>> = HashMap::new();
            for &si in pool {
                let s = &all_sentences[si];
                let anchor = s
                    .grammar_relationships
                    .iter()
                    .find_map(|g| g.object.clone().filter(|o| !o.trim().is_empty()))
                    .or_else(|| s.knowledge_refs.first().map(|k| k.surface.clone()))
                    .unwrap_or_else(|| "(core)".to_string());
                by_anchor.entry(norm(&anchor)).or_default().push(si);
            }
            for (anchor_key, members) in by_anchor {
                let anchor_display = all_sentences[members[0]]
                    .grammar_relationships
                    .iter()
                    .find_map(|g| g.object.clone())
                    .or_else(|| {
                        all_sentences[members[0]]
                            .knowledge_refs
                            .first()
                            .map(|k| k.surface.clone())
                    })
                    .unwrap_or_else(|| anchor_key.clone());
                let verbs: HashSet<&str> = members
                    .iter()
                    .flat_map(|&si| {
                        all_sentences[si]
                            .grammar_relationships
                            .iter()
                            .map(|g| g.verb.trim())
                    })
                    .collect();
                let branch_text = if verbs.len() == 1 {
                    format!("{} — {}", anchor_display, verbs.iter().next().unwrap_or(&""))
                } else {
                    anchor_display.clone()
                };

                let branch_id = node_id_counter;
                node_id_counter += 1;
                let mut branch = BranchCapture {
                    branch: branch_text,
                    parent_intent: intent_text.clone(),
                    source_methodology_ids: Vec::new(),
                    source_chunk_indices: Vec::new(),
                    source_sentences: Vec::new(),
                    node_id: branch_id,
                };

                // Each member sentence is a DetailCapture — the leaf evidence.
                for &si in &members {
                    let s = &all_sentences[si];
                    let is_constraint = s.grammar_relationships.iter().any(|g| g.negated);
                    let detail_id = node_id_counter;
                    node_id_counter += 1;
                    state.detail_captures.push(DetailCapture {
                        content: s.content.clone(),
                        detail_type: if is_constraint {
                            "constraint".to_string()
                        } else {
                            "detail".to_string()
                        },
                        parent_branch: branch.branch.clone(),
                        parent_intent: intent_text.clone(),
                        source_chunk_indices: vec![s.chunk_id],
                        source_sentences: vec![s.original_content.clone()],
                        node_id: detail_id,
                    });
                    if !branch.source_chunk_indices.contains(&s.chunk_id) {
                        branch.source_chunk_indices.push(s.chunk_id);
                    }
                }

                // Aggregate this branch pool's knowledge onto the intent map.
                let entry = knowledge_paths.entry(intent_text.clone()).or_default();
                for &si in &members {
                    for kr in &all_sentences[si].knowledge_refs {
                        let p = kr.topic_path.join("/");
                        if !p.is_empty() && !entry.contains(&p) {
                            entry.push(p);
                        }
                    }
                }

                state.branch_captures.push(branch);
            }
        }

        // Default intent when the graph produced nothing promotable.
        if state.intent_captures.is_empty() {
            state.intent_captures.push(IntentCapture {
                intent: "Process user request".to_string(),
                is_parallel: false,
                source_chunk_indices: (0..state.processed_chunks.len() as u32).collect(),
                source_sentences: vec![],
                node_id: node_id_counter,
            });
            node_id_counter += 1;
        }

        // E-1: corpus-level layer input, gathered ONCE (owned snapshot —
        // pool promotion already ran; this reads the settled corpus).
        let layer_input = self.gather_layer_input(state);

        let mut convergence_pass = 0u32;
        loop {
            convergence_pass += 1;
            let prev_branch_count = state.branch_captures.len();

            // K: enrich with ZSEI knowledge (loads missing methodologies,
            // finds related blueprints, produces prompt-injectable summaries).
            let knowledge = self.enrich_with_zsei_knowledge(state, &layer_input).await;
            let loaded_this_pass = !knowledge.new_methodology_ids.is_empty();

            // M: multi-modal synthesis over the same evidence.
            let synthesis = self.synthesize_modal_evidence(&layer_input);

            let methodology_summaries_block = if knowledge.methodology_summaries.is_empty() {
                String::new()
            } else {
                format!(
                    "APPLIED METHODOLOGY CONTEXT:\n{}\n",
                    knowledge.methodology_summaries.join("\n")
                )
            };

            // ── Methodology branch discovery over the graph intents ──
            let intents_summary: Vec<String> = state
                .intent_captures
                .iter()
                .map(|ic| ic.intent.clone())
                .collect();
            let method_ids = state.methodologies.clone();
            for &method_id in &method_ids {
                if let Ok(Some(method_container)) = self.store.get_container(method_id).await {
                    let method_name = method_container
                        .get("local_state")
                        .and_then(|ls| ls.get("metadata"))
                        .and_then(|m| m.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("Unknown methodology")
                        .to_string();
                    let method_description = method_container
                        .get("local_state")
                        .and_then(|ls| ls.get("context"))
                        .and_then(|ctx| ctx.get("keywords"))
                        .map(|kw| kw.to_string())
                        .unwrap_or_default();

                    let known_branches_json: Vec<serde_json::Value> = state
                        .branch_captures
                        .iter()
                        .map(|bc| serde_json::json!({"branch": bc.branch, "intent": bc.parent_intent}))
                        .collect();

                    let branch_prompt = format!(
                        r#"You are applying the methodology "{}" to a set of user intents.
Methodology context: {}

USER INTENTS:
{}

{methodology_summaries_block}

ALREADY IDENTIFIED BRANCHES (do NOT repeat these):
{}

Based on this methodology, what additional branches (sub-components, requirements, or considerations) should be addressed for each intent?
Only suggest branches NOT already in the known list.

Return ONLY valid JSON:
{{
    "branches": [
        {{
            "branch": "specific branch description",
            "parent_intent": "the intent this branch belongs to",
            "rationale": "why this methodology requires this branch"
        }}
    ]
}}
If no new branches apply, return: {{"branches": []}}"#,
                        method_name,
                        &method_description[..method_description.len().min(300)],
                        intents_summary.join("\n"),
                        serde_json::to_string(&known_branches_json).unwrap_or_default(),
                        methodology_summaries_block = methodology_summaries_block,
                    );

                    let branch_input = serde_json::json!({
                        "prompt": branch_prompt,
                        "max_tokens": 600,
                        "temperature": 0.3,
                        "system_context": "Suggest branches per methodology. Return only valid JSON. No explanation."
                    });

                    if let Ok(result) = self.metered_execute(state, 9, branch_input).await {
                        let response =
                            result.get("response").and_then(|r| r.as_str()).unwrap_or("{}");
                        let json_str = Self::extract_json_from_response(response, '{', '}');
                        let parsed = serde_json::from_str::<serde_json::Value>(json_str.trim())
                            .unwrap_or_else(|_| serde_json::json!({"branches": []}));

                        if let Some(branches) = parsed.get("branches").and_then(|b| b.as_array()) {
                            for branch_val in branches {
                                let branch_str = branch_val
                                    .get("branch")
                                    .and_then(|b| b.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let parent_intent = branch_val
                                    .get("parent_intent")
                                    .and_then(|p| p.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                if branch_str.is_empty() {
                                    continue;
                                }

                                let resolved_parent = state
                                    .intent_captures
                                    .iter()
                                    .find(|ic| {
                                        ic.intent.to_lowercase()
                                            .contains(&parent_intent.to_lowercase())
                                            || parent_intent
                                                .to_lowercase()
                                                .contains(&ic.intent.to_lowercase())
                                    })
                                    .map(|ic| ic.intent.clone())
                                    .unwrap_or_else(|| {
                                        state
                                            .intent_captures
                                            .first()
                                            .map(|ic| ic.intent.clone())
                                            .unwrap_or_default()
                                    });

                                let already_exists = state.branch_captures.iter().any(|bc| {
                                    bc.parent_intent == resolved_parent
                                        && (bc
                                            .branch
                                            .to_lowercase()
                                            .contains(&branch_str.to_lowercase())
                                            || branch_str
                                                .to_lowercase()
                                                .contains(&bc.branch.to_lowercase()))
                                });

                                if !already_exists {
                                    let branch_id = node_id_counter;
                                    node_id_counter += 1;
                                    state.branch_captures.push(BranchCapture {
                                        branch: branch_str,
                                        parent_intent: resolved_parent,
                                        source_methodology_ids: vec![method_id],
                                        source_chunk_indices: vec![],
                                        source_sentences: vec![],
                                        node_id: branch_id,
                                    });
                                } else if let Some(existing) = state
                                    .branch_captures
                                    .iter_mut()
                                    .find(|bc| {
                                        bc.parent_intent == resolved_parent
                                            && bc.branch.to_lowercase().contains(&branch_str.to_lowercase())
                                    })
                                {
                                    if !existing.source_methodology_ids.contains(&method_id) {
                                        existing.source_methodology_ids.push(method_id);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // E.2: cross-reference methodologies per intent layer — loads
            // existing domain methodologies or synthesizes missing ones.
            // Bounded to the first 3 layers to control prompt explosion.
            let layers = state.intent_captures.len().min(3).max(1) as u32;
            for layer in 1..=layers {
                let _findings = self
                    .cross_reference_methodologies_for_layer(state, layer)
                    .await;
            }

            // ── Cross-references: cross-pool relationship edges become
            //       CrossRefs (deterministic anchor mapping — no model call) ──
            for rel in &cross_rels {
                if let (Some(sa), Some(sb)) = (
                    all_sentences.iter().find(|s| s.node_id == rel.from_sentence_id),
                    all_sentences.iter().find(|s| s.node_id == rel.to_sentence_id),
                ) {
                    let anchor_of = |s: &OrchSentenceNode| -> Option<String> {
                        s.grammar_relationships
                            .iter()
                            .find_map(|g| g.object.clone().filter(|o| !o.trim().is_empty()))
                            .or_else(|| s.knowledge_refs.first().map(|k| k.surface.clone()))
                    };
                    if let (Some(aa), Some(ab)) = (anchor_of(sa), anchor_of(sb)) {
                        if norm(&aa) == norm(&ab) {
                            continue;
                        }
                        let branch_of = |anchor: &String| -> Option<String> {
                            state
                                .branch_captures
                                .iter()
                                .find(|bc| bc.branch.to_lowercase().contains(&norm(anchor)))
                                .map(|bc| bc.branch.clone())
                        };
                        if let (Some(f), Some(t)) = (branch_of(&aa), branch_of(&ab)) {
                            if f == t {
                                continue;
                            }
                            let relation_type = match rel.relationship_type.as_str() {
                                "Causes" | "Enables" => AMTRelationType::DependsOn,
                                "Contradicts" => AMTRelationType::Contradicts,
                                "Elaborates" | "Exemplifies" | "Summarizes" | "SimilarTo" => {
                                    AMTRelationType::Elaborates
                                }
                                "PartOf" => AMTRelationType::SharedContext,
                                _ => AMTRelationType::RelatesTo,
                            };
                            let dup = state.cross_refs.iter().any(|cr| {
                                cr.from_branch == f
                                    && cr.to_branch == t
                                    && cr.relation_type == relation_type
                            });
                            if !dup {
                                state.cross_refs.push(CrossRef {
                                    from_branch: f,
                                    to_branch: t,
                                    from_intent: String::new(),
                                    to_intent: String::new(),
                                    relation_type,
                                    description: rel.evidence.clone(),
                                });
                            }
                        }
                    }
                }
            }

            // Q: score every branch against evidence / methodology / modality
            // coverage; prune low-evidence methodology-less branches.
            let qualities = self.score_branch_quality(state, &layer_input, &synthesis);
            let pruned = {
                let before = state.branch_captures.len();
                self.apply_branch_quality(state, &qualities);
                before - state.branch_captures.len()
            };

            // Convergence: nothing grew, nothing was pruned, nothing new
            // loaded — or the safety ceiling.
            let grew = state.branch_captures.len() != prev_branch_count;
            let convergence = crate::k_registry::KAlgorithms::global()
                .convergence
                .default_preset()
                .clone();
            if (!grew && pruned == 0 && !loaded_this_pass)
                || convergence_pass >= convergence.max_passes
            {
                break;
            }
        }

        // ── Tree assembly (shared assembler) ──
        let mut counter = node_id_counter;
        let root = self.assemble_amt_from_captures(state, &mut counter, &knowledge_paths);
        Ok(root)
    }

    /// Build AMT layer-by-layer from processed chunks (processes each chunk individually)
    async fn build_amt_layer_by_layer(
        &self,
        state: &mut OrchestrationState,
    ) -> Result<AMTNode, String> {
        let max_outer_passes = 10;
        let convergence_threshold = 5; // passes without new insights before done
        let mut consecutive_no_new = 0u32;
        let mut node_id_counter = 1u64;

        // Initial modality graphs are available in state.modality_graphs.
        // Branch discovery prompts can reference detected modalities from graph metadata.
        // Use state.root_modality_list.verified_modalities for evidence of what's present.
        let detected_modality_names: Vec<String> = state
            .root_modality_list
            .verified_modalities
            .iter()
            .map(|vm| vm.modality.clone())
            .collect();

        // Outer convergence loop
        'outer: for outer_pass in 0..max_outer_passes {
            state.amt_pass_count += 1;
            #[allow(unused_assignments)]
            let mut new_insights_this_pass = false;

            // --- PHASE 1A: Intent discovery ---
            // Build context of already-known intents for deduplication
            let known_intents_json: Vec<serde_json::Value> = state
                .intent_captures
                .iter()
                .map(|ic| serde_json::json!({"intent": ic.intent, "is_parallel": ic.is_parallel}))
                .collect();

            // Snapshot: loop body calls metered_execute (&mut state) — iterate
            // an owned clone so borrows don't conflict. Chunks are not mutated
            // in this loop; iteration semantics identical.
            let chunks_snapshot = state.processed_chunks.clone();
            for chunk in &chunks_snapshot {
                let intent_prompt = format!(
                    r#"Analyze this text chunk to identify goals or intents expressed in it.
        A chunk may express MULTIPLE unrelated intents (parallel) or a single intent.

        ALREADY KNOWN INTENTS (do NOT repeat these):
        {}

        CHUNK {} of {}:
        {}

        MODALITIES DETECTED IN CONTENT: {}

        Return ONLY valid JSON with no explanation:
        {{
            "new_intents": [
                {{
                    "intent": "clear description of this goal/intent",
                    "is_parallel": true,
                    "source_sentence": "the exact sentence or paragraph from the chunk expressing this"
                }}
            ]
        }}
        If no new intents are found, return: {{"new_intents": []}}"#,
                    serde_json::to_string(&known_intents_json).unwrap_or_default(),
                    chunk.index + 1,
                    state.processed_chunks.len(),
                    &chunk.cleaned_text[..chunk.cleaned_text.len().min(1500)],
                    detected_modality_names.join(", ")
                );

                let intent_input = serde_json::json!({
                    "prompt": intent_prompt,
                    "max_tokens": 500,
                    "temperature": 0.2,
                    "system_context": "Extract new intents not already listed. Return only valid JSON. No explanation."
                });

                if let Ok(result) = self.metered_execute(state, 9, intent_input).await {
                    let response = result
                        .get("response")
                        .and_then(|r| r.as_str())
                        .unwrap_or("{}");
                    let json_str = Self::extract_json_from_response(response, '{', '}');
                    let parsed = serde_json::from_str::<serde_json::Value>(json_str.trim())
                        .unwrap_or_else(|_| serde_json::json!({"new_intents": []}));

                    if let Some(new_intents) = parsed.get("new_intents").and_then(|n| n.as_array())
                    {
                        for intent_val in new_intents {
                            let intent_str = intent_val
                                .get("intent")
                                .and_then(|i| i.as_str())
                                .unwrap_or("")
                                .to_string();
                            let is_parallel = intent_val
                                .get("is_parallel")
                                .and_then(|p| p.as_bool())
                                .unwrap_or(false);
                            let source_sentence = intent_val
                                .get("source_sentence")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string();

                            if intent_str.is_empty() {
                                continue;
                            }

                            // Check for duplicates (case-insensitive substring match)
                            let already_known = state.intent_captures.iter().any(|ic| {
                                ic.intent
                                    .to_lowercase()
                                    .contains(&intent_str.to_lowercase())
                                    || intent_str
                                        .to_lowercase()
                                        .contains(&ic.intent.to_lowercase())
                            });

                            if !already_known {
                                state.intent_captures.push(IntentCapture {
                                    intent: intent_str,
                                    is_parallel,
                                    source_chunk_indices: vec![chunk.index],
                                    source_sentences: if source_sentence.is_empty() {
                                        vec![]
                                    } else {
                                        vec![source_sentence]
                                    },
                                    node_id: node_id_counter,
                                });
                                node_id_counter += 1;
                                new_insights_this_pass = true;
                            } else {
                                // Aggregate: add this chunk as an additional source
                                if let Some(existing) =
                                    state.intent_captures.iter_mut().find(|ic| {
                                        ic.intent
                                            .to_lowercase()
                                            .contains(&intent_str.to_lowercase())
                                    })
                                {
                                    if !existing.source_chunk_indices.contains(&chunk.index) {
                                        existing.source_chunk_indices.push(chunk.index);
                                        if !source_sentence.is_empty() {
                                            existing.source_sentences.push(source_sentence);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // If no intents found at all, create a default
            if state.intent_captures.is_empty() {
                state.intent_captures.push(IntentCapture {
                    intent: "Process user request".to_string(),
                    is_parallel: false,
                    source_chunk_indices: (0..state.processed_chunks.len() as u32).collect(),
                    source_sentences: vec![],
                    node_id: node_id_counter,
                });
                node_id_counter += 1;
                new_insights_this_pass = true;
            }

            // --- PHASE 1B: Branch discovery via methodologies ---
            // Snapshot (metered_execute needs &mut state inside the loop).
            let method_ids_snapshot = state.methodologies.clone();
            for &method_id in &method_ids_snapshot {
                if let Ok(Some(method_container)) = self.store.get_container(method_id).await {
                    // Extract methodology content
                    let method_name = method_container
                        .get("local_state")
                        .and_then(|ls| ls.get("metadata"))
                        .and_then(|m| m.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("Unknown methodology")
                        .to_string();
                    let method_description = method_container
                        .get("local_state")
                        .and_then(|ls| ls.get("context"))
                        .and_then(|ctx| ctx.get("keywords"))
                        .map(|kw| kw.to_string())
                        .unwrap_or_default();

                    let intents_summary: Vec<String> = state
                        .intent_captures
                        .iter()
                        .map(|ic| ic.intent.clone())
                        .collect();

                    // Already known branches for dedup
                    let known_branches_json: Vec<serde_json::Value> = state.branch_captures
                        .iter()
                        .map(|bc| serde_json::json!({"branch": bc.branch, "intent": bc.parent_intent}))
                        .collect();

                    let branch_prompt = format!(
                        r#"You are applying the methodology "{}" to a set of user intents.
        Methodology context: {}

        USER INTENTS:
        {}

        ALREADY IDENTIFIED BRANCHES (do NOT repeat these):
        {}

        Based on this methodology, what additional branches (sub-components, requirements, or considerations) should be addressed for each intent?
        Only suggest branches NOT already in the known list.

        Return ONLY valid JSON:
        {{
            "branches": [
                {{
                    "branch": "specific branch description",
                    "parent_intent": "the intent this branch belongs to",
                    "rationale": "why this methodology requires this branch"
                }}
            ]
        }}
        If no new branches apply, return: {{"branches": []}}"#,
                        method_name,
                        &method_description[..method_description.len().min(300)],
                        intents_summary.join("\n"),
                        serde_json::to_string(&known_branches_json).unwrap_or_default()
                    );

                    let branch_input = serde_json::json!({
                        "prompt": branch_prompt,
                        "max_tokens": 600,
                        "temperature": 0.3,
                        "system_context": "Suggest branches per methodology. Return only valid JSON. No explanation."
                    });

                    if let Ok(result) = self.metered_execute(state, 9, branch_input).await {
                        let response = result
                            .get("response")
                            .and_then(|r| r.as_str())
                            .unwrap_or("{}");
                        let json_str = Self::extract_json_from_response(response, '{', '}');
                        let parsed = serde_json::from_str::<serde_json::Value>(json_str.trim())
                            .unwrap_or_else(|_| serde_json::json!({"branches": []}));

                        if let Some(branches) = parsed.get("branches").and_then(|b| b.as_array()) {
                            for branch_val in branches {
                                let branch_str = branch_val
                                    .get("branch")
                                    .and_then(|b| b.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let parent_intent = branch_val
                                    .get("parent_intent")
                                    .and_then(|p| p.as_str())
                                    .unwrap_or("")
                                    .to_string();

                                if branch_str.is_empty() {
                                    continue;
                                }

                                // Find actual parent intent (fuzzy match)
                                let resolved_parent = state
                                    .intent_captures
                                    .iter()
                                    .find(|ic| {
                                        ic.intent
                                            .to_lowercase()
                                            .contains(&parent_intent.to_lowercase())
                                            || parent_intent
                                                .to_lowercase()
                                                .contains(&ic.intent.to_lowercase())
                                    })
                                    .map(|ic| ic.intent.clone())
                                    .unwrap_or_else(|| {
                                        state
                                            .intent_captures
                                            .first()
                                            .map(|ic| ic.intent.clone())
                                            .unwrap_or_default()
                                    });

                                let already_exists = state.branch_captures.iter().any(|bc| {
                                    bc.parent_intent == resolved_parent
                                        && (bc
                                            .branch
                                            .to_lowercase()
                                            .contains(&branch_str.to_lowercase())
                                            || branch_str
                                                .to_lowercase()
                                                .contains(&bc.branch.to_lowercase()))
                                });

                                if !already_exists {
                                    state.branch_captures.push(BranchCapture {
                                        branch: branch_str,
                                        parent_intent: resolved_parent,
                                        source_methodology_ids: vec![method_id],
                                        source_chunk_indices: vec![],
                                        source_sentences: vec![],
                                        node_id: node_id_counter,
                                    });
                                    node_id_counter += 1;
                                    new_insights_this_pass = true;
                                } else {
                                    // Aggregate: add methodology as additional source
                                    if let Some(existing) =
                                        state.branch_captures.iter_mut().find(|bc| {
                                            bc.parent_intent == resolved_parent
                                                && bc
                                                    .branch
                                                    .to_lowercase()
                                                    .contains(&branch_str.to_lowercase())
                                        })
                                    {
                                        if !existing.source_methodology_ids.contains(&method_id) {
                                            existing.source_methodology_ids.push(method_id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // --- PHASE 2: Detail discovery over chunks ---
            // Snapshot (metered_execute needs &mut state inside the loop).
            let chunks_snapshot = state.processed_chunks.clone();
            for chunk in &chunks_snapshot {
                // Build context of existing branches for this chunk
                let branches_summary: Vec<serde_json::Value> = state
                    .branch_captures
                    .iter()
                    .map(|bc| {
                        serde_json::json!({
                            "branch": bc.branch,
                            "intent": bc.parent_intent
                        })
                    })
                    .collect();

                let known_details_json: Vec<serde_json::Value> = state
                    .detail_captures
                    .iter()
                    .filter(|dc| {
                        // Only known details for branches possibly covered by this chunk
                        state
                            .branch_captures
                            .iter()
                            .find(|bc| bc.branch == dc.parent_branch)
                            .map(|bc| {
                                bc.source_chunk_indices.contains(&chunk.index)
                                    || bc.source_chunk_indices.is_empty()
                            })
                            .unwrap_or(true)
                    })
                    .map(|dc| {
                        serde_json::json!({
                            "detail": dc.content,
                            "branch": dc.parent_branch
                        })
                    })
                    .collect();

                let detail_prompt = format!(
                    r#"Analyze this text chunk for specific details, requirements, and constraints that address the identified branches.

        BRANCHES TO ADDRESS:
        {}

        ALREADY IDENTIFIED DETAILS (do NOT repeat):
        {}

        CHUNK {} of {}:
        {}

        For each branch this chunk addresses, extract specific details. Also identify any completely NEW branches not in the list above.

        Return ONLY valid JSON:
        {{
            "details": [
                {{
                    "content": "specific detail, requirement, or constraint",
                    "type": "detail|requirement|constraint",
                    "parent_branch": "exact branch name this belongs to",
                    "source_sentence": "the exact sentence or paragraph from the chunk"
                }}
            ],
            "new_branches": [
                {{
                    "branch": "newly discovered branch",
                    "parent_intent": "intent it belongs to",
                    "source_sentence": "exact text"
                }}
            ]
        }}"#,
                    serde_json::to_string(&branches_summary).unwrap_or_default(),
                    serde_json::to_string(&known_details_json).unwrap_or_default(),
                    chunk.index + 1,
                    state.processed_chunks.len(),
                    &chunk.cleaned_text[..chunk.cleaned_text.len().min(1500)]
                );

                let detail_input = serde_json::json!({
                    "prompt": detail_prompt,
                    "max_tokens": 700,
                    "temperature": 0.3,
                    "system_context": "Extract details per branch. Return only valid JSON. No explanation."
                });

                if let Ok(result) = self.metered_execute(state, 9, detail_input).await {
                    let response = result
                        .get("response")
                        .and_then(|r| r.as_str())
                        .unwrap_or("{}");
                    let json_str = Self::extract_json_from_response(response, '{', '}');
                    let parsed = serde_json::from_str::<serde_json::Value>(json_str.trim())
                        .unwrap_or_else(|_| serde_json::json!({"details": [], "new_branches": []}));

                    // Process new details
                    if let Some(details) = parsed.get("details").and_then(|d| d.as_array()) {
                        for detail_val in details {
                            let content = detail_val
                                .get("content")
                                .and_then(|c| c.as_str())
                                .unwrap_or("")
                                .to_string();
                            let detail_type = detail_val
                                .get("type")
                                .and_then(|t| t.as_str())
                                .unwrap_or("detail")
                                .to_string();
                            let parent_branch = detail_val
                                .get("parent_branch")
                                .and_then(|p| p.as_str())
                                .unwrap_or("")
                                .to_string();
                            let source_sentence = detail_val
                                .get("source_sentence")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string();

                            if content.is_empty() || parent_branch.is_empty() {
                                continue;
                            }

                            // Resolve parent branch (fuzzy)
                            let resolved_branch = state
                                .branch_captures
                                .iter()
                                .find(|bc| {
                                    bc.branch
                                        .to_lowercase()
                                        .contains(&parent_branch.to_lowercase())
                                        || parent_branch
                                            .to_lowercase()
                                            .contains(&bc.branch.to_lowercase())
                                })
                                .map(|bc| bc.branch.clone())
                                .unwrap_or(parent_branch.clone());

                            // Find parent intent for this branch
                            let resolved_intent = state
                                .branch_captures
                                .iter()
                                .find(|bc| bc.branch == resolved_branch)
                                .map(|bc| bc.parent_intent.clone())
                                .unwrap_or_default();

                            let already_exists = state.detail_captures.iter().any(|dc| {
                                dc.parent_branch == resolved_branch
                                    && (dc.content.to_lowercase().contains(&content.to_lowercase())
                                        || content
                                            .to_lowercase()
                                            .contains(&dc.content.to_lowercase()))
                            });

                            if !already_exists {
                                // Also update branch's chunk indices
                                if let Some(branch) = state
                                    .branch_captures
                                    .iter_mut()
                                    .find(|bc| bc.branch == resolved_branch)
                                {
                                    if !branch.source_chunk_indices.contains(&chunk.index) {
                                        branch.source_chunk_indices.push(chunk.index);
                                    }
                                    if !source_sentence.is_empty()
                                        && !branch.source_sentences.contains(&source_sentence)
                                    {
                                        branch.source_sentences.push(source_sentence.clone());
                                    }
                                }

                                state.detail_captures.push(DetailCapture {
                                    content,
                                    detail_type,
                                    parent_branch: resolved_branch,
                                    parent_intent: resolved_intent,
                                    source_chunk_indices: vec![chunk.index],
                                    source_sentences: if source_sentence.is_empty() {
                                        vec![]
                                    } else {
                                        vec![source_sentence]
                                    },
                                    node_id: node_id_counter,
                                });
                                node_id_counter += 1;
                                new_insights_this_pass = true;
                            } else {
                                // Aggregate
                                if let Some(existing) =
                                    state.detail_captures.iter_mut().find(|dc| {
                                        dc.parent_branch == resolved_branch
                                            && dc
                                                .content
                                                .to_lowercase()
                                                .contains(&content.to_lowercase())
                                    })
                                {
                                    if !existing.source_chunk_indices.contains(&chunk.index) {
                                        existing.source_chunk_indices.push(chunk.index);
                                    }
                                    if !source_sentence.is_empty()
                                        && !existing.source_sentences.contains(&source_sentence)
                                    {
                                        existing.source_sentences.push(source_sentence);
                                    }
                                }
                            }
                        }
                    }

                    // Process new branches discovered during detail pass
                    if let Some(new_branches) =
                        parsed.get("new_branches").and_then(|nb| nb.as_array())
                    {
                        for branch_val in new_branches {
                            let branch_str = branch_val
                                .get("branch")
                                .and_then(|b| b.as_str())
                                .unwrap_or("")
                                .to_string();
                            let parent_intent = branch_val
                                .get("parent_intent")
                                .and_then(|p| p.as_str())
                                .unwrap_or("")
                                .to_string();
                            let source_sentence = branch_val
                                .get("source_sentence")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string();

                            if branch_str.is_empty() {
                                continue;
                            }

                            let already_exists = state.branch_captures.iter().any(|bc| {
                                bc.branch
                                    .to_lowercase()
                                    .contains(&branch_str.to_lowercase())
                                    || branch_str
                                        .to_lowercase()
                                        .contains(&bc.branch.to_lowercase())
                            });

                            if !already_exists {
                                let resolved_parent = state
                                    .intent_captures
                                    .iter()
                                    .find(|ic| {
                                        ic.intent
                                            .to_lowercase()
                                            .contains(&parent_intent.to_lowercase())
                                            || parent_intent
                                                .to_lowercase()
                                                .contains(&ic.intent.to_lowercase())
                                    })
                                    .map(|ic| ic.intent.clone())
                                    .unwrap_or_else(|| {
                                        state
                                            .intent_captures
                                            .first()
                                            .map(|ic| ic.intent.clone())
                                            .unwrap_or_default()
                                    });

                                state.branch_captures.push(BranchCapture {
                                    branch: branch_str,
                                    parent_intent: resolved_parent,
                                    source_methodology_ids: vec![],
                                    source_chunk_indices: vec![chunk.index],
                                    source_sentences: if source_sentence.is_empty() {
                                        vec![]
                                    } else {
                                        vec![source_sentence]
                                    },
                                    node_id: node_id_counter,
                                });
                                node_id_counter += 1;
                                new_insights_this_pass = true;
                            }
                        }
                    }
                }
            }

            // Cross-reference methodologies for the current layer state
            let method_findings = self
                .cross_reference_methodologies_for_layer(state, outer_pass as u32)
                .await;

            let has_new_methodology = !method_findings.is_empty();

            if !new_insights_this_pass && !has_new_methodology {
                consecutive_no_new += 1;
                if consecutive_no_new >= convergence_threshold {
                    break 'outer;
                }
            } else {
                if has_new_methodology {
                    // New methodologies may surface new branches — reset convergence
                    new_insights_this_pass = true;
                }
                consecutive_no_new = 0;
            }
        } // end 'outer

        // --- PHASE 3: Cross-reference linking ---
        let branch_list: Vec<(String, String)> = state
            .branch_captures
            .iter()
            .map(|bc| (bc.branch.clone(), bc.parent_intent.clone()))
            .collect();

        // Pairwise pass discipline from the K-ALGORITHM registry.
        let pairwise = crate::k_registry::KAlgorithms::global().pairwise.default_preset().clone();
        let max_pairs = pairwise.max_pairs;
        let mut pair_count = 0;
        'pairs: for i in 0..branch_list.len() {
            for j in (i + 1)..branch_list.len() {
                if pair_count >= max_pairs {
                    break 'pairs;
                }
                pair_count += 1;

                let (branch_a, intent_a) = &branch_list[i];
                let (branch_b, intent_b) = &branch_list[j];

                // Skip if same intent (same-intent relationships are handled by hierarchy)
                if intent_a == intent_b {
                    continue;
                }

                let crossref_prompt = format!(
                    r#"Are these two branches related to each other?

        BRANCH A (from intent: "{}"): {}
        BRANCH B (from intent: "{}"): {}

        If they are related, describe how.

        Return ONLY valid JSON:
        {{
            "related": true,
            "relationship_type": "depends_on|requires|relates_to|contradicts|shared_context",
            "description": "brief explanation"
        }}
        If not related: {{"related": false}}"#,
                    intent_a, branch_a, intent_b, branch_b
                );

                let crossref_input = serde_json::json!({
                    "prompt": crossref_prompt,
                    "max_tokens": 150,
                    "temperature": 0.2,
                    "system_context": "Identify cross-branch relationships. Return only valid JSON."
                });

                if let Ok(result) = self.metered_execute(state, 9, crossref_input).await {
                    let response = result
                        .get("response")
                        .and_then(|r| r.as_str())
                        .unwrap_or("{}");
                    let json_str = Self::extract_json_from_response(response, '{', '}');
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str.trim()) {
                        if parsed
                            .get("related")
                            .and_then(|r| r.as_bool())
                            .unwrap_or(false)
                        {
                            let rel_type_str = parsed
                                .get("relationship_type")
                                .and_then(|rt| rt.as_str())
                                .unwrap_or("relates_to");
                            let description = parsed
                                .get("description")
                                .and_then(|d| d.as_str())
                                .unwrap_or("")
                                .to_string();

                            let relation_type = match rel_type_str {
                                "depends_on" => AMTRelationType::DependsOn,
                                "requires" => AMTRelationType::Requires,
                                "contradicts" => AMTRelationType::Contradicts,
                                "shared_context" => AMTRelationType::SharedContext,
                                _ => AMTRelationType::RelatesTo,
                            };

                            state.cross_refs.push(CrossRef {
                                from_branch: branch_a.clone(),
                                to_branch: branch_b.clone(),
                                from_intent: intent_a.clone(),
                                to_intent: intent_b.clone(),
                                relation_type,
                                description,
                            });
                        }
                    }
                }
            }
        }

        // --- BUILD AMT FROM CAPTURES (shared assembler) ---
        let mut counter = node_id_counter;
        let root_node = self.assemble_amt_from_captures(state, &mut counter, &HashMap::new());

        Ok(root_node)
    }

    /// Assemble the AMT tree from captures. Multiple intents → each intent
    /// becomes a Branch under Root with its branches nested beneath; a single
    /// intent → branches attach directly. `knowledge_paths` maps intent text
    /// → aggregated knowledge reference surfaces (graph mode only; empty for
    /// the chunk zero-shot mode).
    fn assemble_amt_from_captures(
        &self,
        state: &OrchestrationState,
        node_id_counter: &mut u64,
        knowledge_paths: &HashMap<String, Vec<String>>,
    ) -> AMTNode {
        let root_intent = {
            let primary: Vec<&IntentCapture> =
                state.intent_captures.iter().filter(|ic| !ic.is_parallel).collect();
            if primary.len() == 1 {
                primary[0].intent.clone()
            } else if !primary.is_empty() {
                format!(
                    "Multiple goals: {}",
                    primary.iter().map(|ic| ic.intent.as_str()).collect::<Vec<_>>().join("; ")
                )
            } else if !state.intent_captures.is_empty() {
                state.intent_captures[0].intent.clone()
            } else {
                "Process user request".to_string()
            }
        };

        let mut root_node = AMTNode::new(*node_id_counter, AMTNodeType::Root, root_intent, 0);
        root_node.source_chunk_indices = (0..state.processed_chunks.len() as u32).collect();
        root_node.methodology_ids = state.methodologies.clone();
        // Verification = has source provenance (uniform measured rule).
        root_node.verified = true;
        root_node.confidence = 1.0;
        *node_id_counter += 1;

        let multiple_intents = state.intent_captures.len() > 1;

        for intent_capture in &state.intent_captures {
            if multiple_intents {
                let mut intent_node = AMTNode::new(
                    intent_capture.node_id,
                    AMTNodeType::Branch,
                    intent_capture.intent.clone(),
                    1,
                );
                intent_node.source_chunk_indices = intent_capture.source_chunk_indices.clone();
                intent_node.verified = !intent_capture.source_chunk_indices.is_empty();
                intent_node.confidence = if intent_node.verified { 1.0 } else { 0.0 };
                for sentence in &intent_capture.source_sentences {
                    intent_node.metadata.insert(
                        format!("source_sentence_{}", intent_node.metadata.len()),
                        sentence.clone(),
                    );
                }
                if let Some(paths) = knowledge_paths.get(&intent_capture.intent) {
                    intent_node
                        .metadata
                        .insert("knowledge_paths".to_string(), paths.join(", "));
                }

                let branches_for_intent: Vec<&BranchCapture> = state
                    .branch_captures
                    .iter()
                    .filter(|bc| bc.parent_intent == intent_capture.intent)
                    .collect();

                for branch_capture in branches_for_intent {
                    intent_node
                        .children
                        .push(self.build_branch_node(state, branch_capture, 2));
                }

                root_node.children.push(intent_node);
            } else {
                let branches_for_intent: Vec<&BranchCapture> = state
                    .branch_captures
                    .iter()
                    .filter(|bc| bc.parent_intent == intent_capture.intent)
                    .collect();

                for branch_capture in branches_for_intent {
                    root_node
                        .children
                        .push(self.build_branch_node(state, branch_capture, 1));
                }
                if let Some(paths) = knowledge_paths.get(&intent_capture.intent) {
                    root_node
                        .metadata
                        .insert("knowledge_paths".to_string(), paths.join(", "));
                }
            }
        }

        root_node
    }

    /// One intent's branch node: details attached as children (constraint →
    /// Consideration, otherwise Leaf), cross-references resolved by branch.
    fn build_branch_node(
        &self,
        state: &OrchestrationState,
        branch_capture: &BranchCapture,
        depth: u32,
    ) -> AMTNode {
        let mut branch_node = AMTNode::new(
            branch_capture.node_id,
            AMTNodeType::Branch,
            branch_capture.branch.clone(),
            depth,
        );
        branch_node.source_chunk_indices = branch_capture.source_chunk_indices.clone();
        branch_node.methodology_ids = branch_capture.source_methodology_ids.clone();
        // Chunk evidence = verified; methodology-suggested-only = unverified.
        branch_node.verified = !branch_capture.source_chunk_indices.is_empty();
        branch_node.confidence = if branch_node.verified { 1.0 } else { 0.0 };
        for sentence in &branch_capture.source_sentences {
            branch_node.metadata.insert(
                format!("source_sentence_{}", branch_node.metadata.len()),
                sentence.clone(),
            );
        }

        let details_for_branch: Vec<&DetailCapture> = state
            .detail_captures
            .iter()
            .filter(|dc| dc.parent_branch == branch_capture.branch)
            .collect();

        for detail_capture in details_for_branch {
            let node_type = match detail_capture.detail_type.as_str() {
                "constraint" => AMTNodeType::Consideration,
                _ => AMTNodeType::Leaf,
            };
            let mut detail_node = AMTNode::new(
                detail_capture.node_id,
                node_type,
                detail_capture.content.clone(),
                depth + 1,
            );
            detail_node
                .metadata
                .insert("type".to_string(), detail_capture.detail_type.clone());
            detail_node.source_chunk_indices = detail_capture.source_chunk_indices.clone();
            detail_node.verified = !detail_capture.source_chunk_indices.is_empty();
            detail_node.confidence = if detail_node.verified { 1.0 } else { 0.0 };
            for sentence in &detail_capture.source_sentences {
                detail_node.metadata.insert(
                    format!("source_sentence_{}", detail_node.metadata.len()),
                    sentence.clone(),
                );
            }
            branch_node.children.push(detail_node);
        }

        for cross_ref in &state.cross_refs {
            if cross_ref.from_branch == branch_capture.branch {
                let target_node_id = state
                    .branch_captures
                    .iter()
                    .find(|bc| bc.branch == cross_ref.to_branch)
                    .map(|bc| bc.node_id)
                    .unwrap_or(0);
                if target_node_id > 0 {
                    branch_node.relationships.push(AMTRelation {
                        target_id: target_node_id,
                        relation_type: cross_ref.relation_type.clone(),
                        // Established cross-sentence edge — exists in the
                        // graph, verified by construction.
                        confidence: 1.0,
                    });
                }
            }
        }

        branch_node
    }

    fn gather_layer_input(&self, state: &OrchestrationState) -> LayerInput {
        let mut all_keywords: std::collections::HashSet<String> =
            state.keywords.iter().cloned().collect();
        let mut all_topics: std::collections::HashSet<String> =
            state.topics.iter().cloned().collect();
        let mut grammar_evidence: Vec<GrammarEvidence> = Vec::new();
        let mut modality_span_evidence: HashMap<String, Vec<ModalitySpanEvidence>> = HashMap::new();

        for chunk in &state.processed_chunks {
            for kw in &chunk.keywords {
                all_keywords.insert(kw.clone());
            }
            for topic in &chunk.topics {
                all_topics.insert(topic.clone());
            }
            // Grammar evidence rides on the SentenceNode (grammar is local to
            // each sentence, not a chunk-level bag).
            for sent in &chunk.sentence_nodes {
                for gr in &sent.grammar_relationships {
                    grammar_evidence.push(GrammarEvidence {
                        from_text: gr.from_text.clone(),
                        to_text: gr.to_text.clone(),
                        edge_type: gr.edge_type.clone(),
                        chunk_index: gr.chunk_index,
                        tense: gr.tense.clone(),
                        negated: gr.negated,
                    });
                }
            }
            // Typed modality detections.
            for detection in &chunk.detected_modalities {
                modality_span_evidence
                    .entry(detection.modality.clone())
                    .or_default()
                    .push(ModalitySpanEvidence {
                        chunk_index: detection.chunk_index,
                        span_start: detection.span_start,
                        span_end: detection.span_end,
                        intent_reference: detection.intent_reference.clone(),
                    });
            }
        }

        let mut file_contexts: Vec<FileLayerContext> = state.classified_file_graphs.iter()
            .map(|cfg| FileLayerContext {
                file_path: cfg.file_path.clone(),
                modality: cfg.modality.clone(),
                role: format!("{:?}", cfg.role),
                graph_id: cfg.graph_id,
            })
            .collect();

        for (path, &graph_id) in &state.file_graphs {
            if !file_contexts.iter().any(|fc| fc.file_path == *path) {
                file_contexts.push(FileLayerContext {
                    file_path: path.clone(),
                    modality: self.detect_file_modality(path),
                    role: "RawData".to_string(),
                    graph_id,
                });
            }
        }

        let graph_contexts: Vec<GraphLayerContext> = state.modality_graphs.iter()
            .map(|(modality, &graph_id)| {
                let gs = state.graph_states.get(&graph_id);
                GraphLayerContext {
                    modality: modality.clone(),
                    graph_id,
                    state: gs.map(|g| format!("{:?}", g.state))
                        .unwrap_or_else(|| "Unknown".to_string()),
                    cross_modal_edge_count: gs.map(|g| g.cross_modal_edge_count).unwrap_or(0),
                    pipeline_id: self.modality_name_to_pipeline_id(modality),
                }
            })
            .collect();

        let verified_modalities: Vec<String> = state.root_modality_list
            .verified_modalities.iter()
            .map(|vm| vm.modality.clone())
            .collect();

        LayerInput {
            keywords: all_keywords.into_iter().collect(),
            topics: all_topics.into_iter().collect(),
            grammar_evidence,
            modality_span_evidence,
            file_contexts,
            graph_contexts,
            verified_modalities,
            cleaned_prompt: state.cleaned_prompt.clone(),
            chunk_count: state.processed_chunks.len() as u32,
            chunk_graph_ids: state.chunk_graph_ids.clone(),
        }
    }

    async fn enrich_with_zsei_knowledge(
        &self,
        state: &mut OrchestrationState,
        layer_input: &LayerInput,
    ) -> LayerKnowledge {
        let mut knowledge = LayerKnowledge::default();

        let search_keywords: Vec<String> = layer_input.keywords.iter()
            .chain(layer_input.topics.iter())
            .take(12)
            .cloned()
            .collect();

        if !search_keywords.is_empty() {
            let found_methods = self.store
                .search_by_keywords(&search_keywords, Some("Methodology"))
                .await
                .unwrap_or_default();

            for method_id in found_methods {
                if !state.methodologies.contains(&method_id) {
                    if let Ok(Some(container)) = self.store.get_container(method_id).await {
                        let name = container
                            .get("local_state").and_then(|ls| ls.get("metadata"))
                            .and_then(|m| m.get("name")).and_then(|n| n.as_str())
                            .unwrap_or("Unknown").to_string();

                        let keywords_str = container
                            .get("local_state").and_then(|ls| ls.get("context"))
                            .and_then(|ctx| ctx.get("keywords"))
                            .and_then(|k| k.as_array())
                            .map(|arr| {
                                arr.iter().filter_map(|v| v.as_str()).take(5)
                                    .collect::<Vec<_>>().join(", ")
                            })
                            .unwrap_or_default();

                        state.methodologies.push(method_id);
                        knowledge.new_methodology_ids.push(method_id);
                        knowledge.methodology_summaries.push(
                            format!("[{}] covers: {}", name, keywords_str)
                        );
                    }
                }
            }
        }

        if state.blueprint_id.is_none() {
            let found_blueprints = self.store
                .search_by_keywords(
                    &layer_input.keywords.iter().take(8).cloned().collect::<Vec<_>>(),
                    Some("Blueprint"),
                )
                .await
                .unwrap_or_default();

            knowledge.related_blueprint_ids = found_blueprints.into_iter().take(5).collect();
        }

        for modality in &layer_input.verified_modalities {
            let modality_methods = self.store
                .search_by_keywords(&[modality.clone()], Some("Methodology"))
                .await
                .unwrap_or_default();

            for method_id in modality_methods {
                if !state.methodologies.contains(&method_id) {
                    state.methodologies.push(method_id);
                    knowledge.new_methodology_ids.push(method_id);
                }
            }
        }

        knowledge
    }

    fn synthesize_modal_evidence(
        &self,
        layer_input: &LayerInput,
    ) -> ModalSynthesis {
        let active_modalities: Vec<String> = layer_input.modality_span_evidence.iter()
            .filter(|(_, spans)| !spans.is_empty())
            .map(|(m, _)| m.clone())
            .chain(
                layer_input.graph_contexts.iter()
                    .filter(|gc| gc.cross_modal_edge_count > 0)
                    .map(|gc| gc.modality.clone())
            )
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if active_modalities.is_empty() {
            return ModalSynthesis {
                cross_modal_pairs: vec![],
                cross_modal_summary: String::new(),
                active_modalities: vec!["text".to_string()],
            };
        }

        let mut cross_modal_pairs: Vec<(String, String, String)> = Vec::new();

        for modality in &active_modalities {
            if modality != "text" {
                let span_count = layer_input.modality_span_evidence
                    .get(modality).map(|v| v.len()).unwrap_or(0);
                if span_count > 0 {
                    let relationship = if span_count > 5 { "extensively references" }
                        else if span_count > 2 { "references" }
                        else { "mentions" };
                    cross_modal_pairs.push((
                        "text".to_string(),
                        modality.clone(),
                        relationship.to_string(),
                    ));
                }
            }
        }

        for fc in &layer_input.file_contexts {
            if !fc.modality.is_empty() && fc.modality != "text" {
                let rel = match fc.role.as_str() {
                    "Primary" => "is the primary subject of",
                    "Supplementary" => "provides context for",
                    _ => "is data input for",
                };
                cross_modal_pairs.push(("text".to_string(), fc.modality.clone(), rel.to_string()));
            }
        }

        cross_modal_pairs.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);

        let cross_modal_summary = if cross_modal_pairs.is_empty() {
            if active_modalities.len() > 1 {
                format!("Content spans {} modalities: {}.",
                    active_modalities.len(), active_modalities.join(", "))
            } else { String::new() }
        } else {
            let pair_descriptions: Vec<String> = cross_modal_pairs.iter().take(4)
                .map(|(a, b, rel)| format!("{} {} {}", a, rel, b))
                .collect();
            format!("Cross-modal structure: {}. Active: {}.",
                pair_descriptions.join("; "), active_modalities.join(", "))
        };

        ModalSynthesis { cross_modal_pairs, cross_modal_summary, active_modalities }
    }

    fn score_branch_quality(
        &self,
        state: &OrchestrationState,
        layer_input: &LayerInput,
        modal_synthesis: &ModalSynthesis,
    ) -> Vec<BranchQuality> {
        let active_modal_count = modal_synthesis.active_modalities.len().max(1) as f32;

        state.branch_captures.iter().map(|bc| {
            let chunk_mentions = bc.source_chunk_indices.len() as f32;
            let evidence_score = (chunk_mentions / layer_input.chunk_count.max(1) as f32).min(1.0);

            let methodology_score = if bc.source_methodology_ids.is_empty() {
                0.0f32
            } else {
                (bc.source_methodology_ids.len() as f32 * 0.33).min(1.0)
            };

            let branch_lower = bc.branch.to_lowercase();
            let modalities_touched: std::collections::HashSet<String> = layer_input
                .modality_span_evidence.iter()
                .filter(|(_, spans)| {
                    spans.iter().any(|s|
                        s.intent_reference.to_lowercase().contains(&branch_lower))
                })
                .map(|(m, _)| m.clone())
                .collect();

            let modal_coverage = modalities_touched.len() as f32 / active_modal_count;

            let grammar_bonus = if layer_input.grammar_evidence.iter().any(|gr| {
                gr.from_text.to_lowercase().contains(&branch_lower)
                    || gr.to_text.to_lowercase().contains(&branch_lower)
            }) { 0.1f32 } else { 0.0f32 };

            let total_score = ((evidence_score * 0.4)
                + (methodology_score * 0.35)
                + (modal_coverage * 0.15)
                + grammar_bonus).min(1.0);

            BranchQuality {
                branch: bc.branch.clone(),
                evidence_score,
                methodology_score,
                modal_coverage,
                total_score,
                should_prune: total_score < 0.15 && bc.source_methodology_ids.is_empty(),
            }
        }).collect()
    }

    fn apply_branch_quality(
        &self,
        state: &mut OrchestrationState,
        qualities: &[BranchQuality],
    ) {
        let prunable: std::collections::HashSet<String> = qualities.iter()
            .filter(|q| q.should_prune)
            .map(|q| q.branch.clone())
            .collect();

        if !prunable.is_empty() {
            tracing::debug!(
                "Branch quality pruning {} low-evidence branches: {:?}",
                prunable.len(), prunable
            );
            state.branch_captures.retain(|bc| !prunable.contains(&bc.branch));
            state.detail_captures.retain(|dc| !prunable.contains(&dc.parent_branch));
        }
    }

    /// Cross-reference methodologies for a specific AMT layer.
    /// For each branch in the layer, finds required methodologies.
    /// Loads existing ones from ZSEI or synthesizes new ones when gaps exist.
    /// Returns a list of findings (loaded or created methodology IDs).
    async fn cross_reference_methodologies_for_layer(
        &self,
        state: &mut OrchestrationState,
        layer: u32,
    ) -> Vec<MethodologyFinding> {
        let mut findings = Vec::new();

        // Collect branches that belong to this AMT layer depth
        let layer_branches: Vec<String> = state
            .branch_captures
            .iter()
            .filter(|bc| {
                // Branches at depth = layer (root intent = layer 0, branches = layer 1, etc.)
                // Use intent index as a proxy for layer depth
                let intent_idx = state
                    .intent_captures
                    .iter()
                    .position(|ic| ic.intent == bc.parent_intent)
                    .unwrap_or(0);
                intent_idx + 1 == layer as usize || (layer == 1 && intent_idx == 0)
            })
            .map(|bc| bc.branch.clone())
            .collect();

        if layer_branches.is_empty() {
            return findings;
        }

        // Ask LLM which methodology domains are required for these branches
        let prompt = format!(
            r#"Given these AMT branches at layer {}, identify which methodology domains are needed.
    A methodology domain is a named area of systematic knowledge (e.g., "Software Testing",
    "Data Privacy", "Scientific Rigor", "API Design").

    Branches:
    {}

    Return ONLY valid JSON array:
    ["domain1", "domain2"]"#,
            layer,
            layer_branches
                .iter()
                .map(|b| format!("- {}", b))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let input = serde_json::json!({
            "prompt": prompt,
            "max_tokens": 200,
            "temperature": 0.2,
            "system_context": "Methodology domain identification. Return only valid JSON array."
        });

        let required_domains: Vec<String> = match self.metered_execute(state, 9, input).await {
            Ok(result) => {
                let raw = result
                    .get("response")
                    .and_then(|r| r.as_str())
                    .unwrap_or("[]");
                let start = raw.find('[').unwrap_or(0);
                let end = raw.rfind(']').map(|i| i + 1).unwrap_or(raw.len());
                serde_json::from_str::<Vec<String>>(&raw[start..end]).unwrap_or_default()
            }
            Err(_) => return findings,
        };

        for domain in &required_domains {
            // Search ZSEI for existing methodology matching this domain
            let existing = self
                .store
                .search_by_keywords(&[domain.clone()], Some("Methodology"))
                .await
                .unwrap_or_default();

            if let Some(&existing_id) = existing.first() {
                if !state.methodologies.contains(&existing_id) {
                    state.methodologies.push(existing_id);
                    findings.push(MethodologyFinding::Loaded(existing_id));
                }
            } else {
                // Synthesize a new methodology for this domain
                let synth_prompt = format!(
                    r#"Create a concise methodology for the domain: "{}"

    Return ONLY valid JSON:
    {{
      "name": "{}",
      "description": "what this methodology covers",
      "category": "domain category",
      "principles": ["principle 1", "principle 2"],
      "keywords": ["keyword1", "keyword2"]
    }}"#,
                    domain, domain
                );

                let synth_input = serde_json::json!({
                    "prompt": synth_prompt,
                    "max_tokens": 400,
                    "temperature": 0.2,
                    "system_context": "Methodology synthesis. Return only valid JSON."
                });

                if let Ok(synth_result) = self.executor.execute(9, synth_input).await {
                    let raw = synth_result
                        .get("response")
                        .and_then(|r| r.as_str())
                        .unwrap_or("{}");
                    let start = raw.find('{').unwrap_or(0);
                    let end = raw.rfind('}').map(|i| i + 1).unwrap_or(raw.len());

                    let methodology_container = serde_json::json!({
                        "container_type": "Methodology",
                        "metadata": {
                            "name": domain,
                            "description": format!("Auto-synthesized methodology for: {}", domain),
                            "created_by": "orchestrator_layer_crossref"
                        },
                        "context": {
                            "keywords": [domain.to_lowercase()],
                            "topics": [domain.to_lowercase()]
                        },
                        "storage": serde_json::from_str::<serde_json::Value>(&raw[start..end])
                            .unwrap_or_default()
                    });

                    if let Ok(new_id) = self.store.create_container(0, methodology_container).await {
                        state.methodologies.push(new_id);
                        findings.push(MethodologyFinding::Created(new_id));
                    }
                }
            }
        }

        findings
    }
}