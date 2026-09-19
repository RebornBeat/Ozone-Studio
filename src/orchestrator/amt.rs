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

        // Persist the completed AMT tree as a real ZSEI container — state.amt
        // previously lived only in this transient per-request
        // OrchestrationState and was discarded the moment orchestrate()
        // returned; only unrelated methodology/blueprint containers were
        // ever created, never the AMT itself, so "what did this run's
        // intent/branch structure look like" was never answerable after the
        // fact. Uses the same in-process StoreAccess the rest of the
        // orchestrator already goes through (no HTTP round-trip needed,
        // unlike pipeline 100 which is a separate process). Task Creation
        // hasn't happened yet at this point in the stage sequence, so the
        // container id is stashed on state and threaded into the task's
        // metadata once the task exists (see stage_6_to_8_execute_steps).
        if state.amt_validated {
            if let Some(amt) = state.amt.clone() {
                match self.persist_amt_container(state, &amt).await {
                    Ok(id) => {
                        state.amt_container_id = Some(id);
                        // Real, observable re-expansion trigger: this
                        // specific persisted AMT still has a node with no
                        // source provenance behind it. Only meaningful for
                        // requests actually anchored to a project — a
                        // one-off chat AMT with no project_id has nowhere
                        // to be "revisited" later.
                        if let Some(project_id) = state.request.project_id {
                            if let Some(unverified) = Self::find_unverified_node(&amt) {
                                Self::record_amt_reexpansion_candidate(
                                    id,
                                    project_id,
                                    &unverified.content,
                                );
                            }
                        }
                    }
                    Err(e) => tracing::warn!("Failed to persist AMT container: {}", e),
                }
            }
        }

        Ok(())
    }

    /// Write the AMT's full node/edge tree to disk (Container has no generic
    /// slot for arbitrary nested tree content, same constraint pipeline
    /// 100's modality graphs hit) and create a real ZSEI container
    /// referencing it, with real keywords/topics from this run's own
    /// extraction — not fabricated placeholders.
    async fn persist_amt_container(
        &self,
        state: &OrchestrationState,
        amt: &AMTNode,
    ) -> Result<u64, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // CONTINUATION (route: Continuation): a subsequent prompt for a
        // project that already has AMT generations chains this tree onto the
        // most recent prior generation via a root-level `Continues` relation
        // (target = the prior AMT's ZSEI container id). Without this, every
        // prompt spawned a sibling AMT island and project history fragmented.
        let mut prior_generation: Option<u64> = None;
        if let Some(project_id) = state.request.project_id {
            if let Some(Some(project_json)) = self
                .store
                .get_container(project_id)
                .await
                .ok()
            {
                let child_ids: Vec<u64> = project_json
                    .get("global_state")
                    .and_then(|g| g.get("child_ids"))
                    .and_then(|c| c.as_array())
                    .map(|a| a.iter().filter_map(|v| v.as_u64()).collect())
                    .unwrap_or_default();
                for child in child_ids.iter().rev() {
                    if let Ok(Some(Some(c))) = self
                        .store
                        .get_container(*child)
                        .await
                        .map(|ok| ok.map(|v| v.as_object().and_then(|o| o.get("local_state")).and_then(|l| l.get("metadata")).and_then(|m| m.get("container_type")).and_then(|t| t.as_str()).map(|s| s.to_string())))
                    {
                        if c == "Derived" {
                            prior_generation = Some(*child);
                            break;
                        }
                    }
                }
            }
        }
        let mut amt = amt.clone();
        // MAIN/FORK designation (the island model): the FIRST AMT in a
        // project is its MAIN tree; every subsequent generation is a FORK —
        // an island spawned from the prior generation, carrying both a
        // `Continues` relation (lineage) and an `amt-fork-of:<id>` keyword
        // (graph-searchable provenance). Merge-back (fork results grafted
        // into main) is the designed v2 — the expansion-candidate routes
        // and the ripple give it its triggers.
        let mut fork_of: Option<u64> = None;
        let mut keywords = state.keywords.clone();
        if let Some(prior) = prior_generation {
            fork_of = Some(prior);
            amt.relationships.push(crate::orchestrator::AMTRelation {
                target_id: prior,
                relation_type: crate::orchestrator::AMTRelationType::Continues,
                confidence: 1.0,
            });
            keywords.push(format!("amt-fork-of:{}", prior));
        } else {
            keywords.push("amt-main".to_string());
        }

        let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
        let amt_dir = format!("{}/amt", data_dir);
        let _ = std::fs::create_dir_all(&amt_dir);
        let file_name = format!("amt_{}_{}.json", now, amt.id);
        let file_path = format!("{}/{}", amt_dir, file_name);
        if let Ok(json) = serde_json::to_string_pretty(&amt) {
            std::fs::write(&file_path, json).map_err(|e| format!("Failed to write AMT tree file: {}", e))?;
        }

        let content_hash = vec![0u8; 32];
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
                    "container_type": "Derived",
                    "modality": "Unknown",
                    "created_at": now,
                    "updated_at": now,
                    "provenance": "orchestrator:amt",
                    "permissions": 0,
                    "owner_id": 0,
                    "name": amt.content.clone(),
                    "materialized_path": null
                },
                "context": {
                    "categories": [],
                    "methodologies": amt.methodology_ids.clone(),
                    "keywords": keywords,
                    "topics": state.topics.clone(),
                    "relationships": [],
                    "learned_associations": [],
                    "embedding": null
                },
                "storage": {
                    "db_shard_id": null,
                    "vector_index_ref": null,
                    "object_store_path": format!("amt/{}", file_name),
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
                    "content_hash": content_hash,
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

        // project_id as real parent — same fix an earlier fork this session
        // applied to text/code modality's persist_graph_container (both had
        // this identical bug: a hardcoded 0 parent that silently discarded
        // a real project_id, so a project's AMT never actually nested under
        // its project container). 0 keeps today's root-parented default for
        // requests with no project_id.
        let parent_id = state.request.project_id.unwrap_or(0);
        let new_id = self.store.create_container(parent_id, container).await?;

        let route = if fork_of.is_some() { "Continuation" } else { "InitialBuild" };
        let _ = crate::orchestrator::amt_candidates::append(
            new_id,
            Some(parent_id),
            route,
            Some(&match fork_of {
                Some(prior) => format!("fork of AMT container {}", prior),
                None => "initial main AMT for this project".to_string(),
            }),
        );

        // MERGE-BACK (task 43 arc): graft substantive branches from this
        // fork into the project's main AMT content file. The main AMT is
        // the earliest-created AMT tree for this project; verified children
        // from this fork that don't exist in main are appended as new
        // branches so the main tree grows across prompts.
        if let Some(project_id) = state.request.project_id {
            self.merge_back_to_main(project_id, &amt).await;
        }

        Ok(new_id)
    }

    /// Graft verified branches from a fork AMT into the project's main AMT
    /// content file. Only grafts branches that don't already exist in main
    /// (content-match dedup). Best-effort — failures logged, never propagated.
    async fn merge_back_to_main(&self, project_id: u64, amt: &AMTNode) {
        let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
        let amt_dir = format!("{}/amt", data_dir);
        let _ = std::fs::create_dir_all(&amt_dir);

        // Find the main AMT file (earliest created for this project).
        let mut main_file: Option<String> = None;
        let mut earliest = u64::MAX;
        if let Ok(entries) = std::fs::read_dir(&amt_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("amt_") && name.ends_with(".json") {
                        if let Ok(metadata) = entry.metadata() {
                            let created = metadata
                                .created()
                                .ok()
                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                .map(|d| d.as_secs())
                                .unwrap_or(u64::MAX);
                            if created < earliest {
                                earliest = created;
                                main_file = Some(path.to_string_lossy().into_owned());
                            }
                        }
                    }
                }
            }
        }

        let Some(main_path) = main_file else {
            tracing::debug!(project_id, "merge-back: no main AMT file found, skipping");
            return;
        };

        let Ok(main_raw) = std::fs::read_to_string(&main_path) else {
            tracing::warn!(project_id, path = %main_path, "merge-back: failed to read main AMT");
            return;
        };
        let mut main_tree: AMTNode = match serde_json::from_str(&main_raw) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(project_id, error = %e, "merge-back: failed to parse main AMT");
                return;
            }
        };

        let mut existing = std::collections::HashSet::new();
        fn collect_contents(node: &AMTNode, set: &mut std::collections::HashSet<String>) {
            set.insert(node.content.clone());
            for child in &node.children {
                collect_contents(child, set);
            }
        }
        collect_contents(&main_tree, &mut existing);

        let mut grafted = 0usize;
        for child in &amt.children {
            if child.verified && !existing.contains(&child.content) {
                main_tree.children.push(child.clone());
                existing.insert(child.content.clone());
                grafted += 1;
            }
        }

        if grafted > 0 {
            let updated = serde_json::to_string_pretty(&main_tree).unwrap_or_default();
            let _ = std::fs::write(&main_path, updated);
            tracing::info!(project_id, grafted, "Merge-back: grafted verified branches into main AMT");
        }
    }

    /// Recorded when a freshly-built AMT still has an unverified node — the
    /// real, observable signal the re-expansion loop (orchestrator/amt_loop.rs)
    /// acts on. Mirrors record_methodology_gap's exact log-file pattern
    /// (append, dedup, cap) rather than inventing a new store-wide scan
    /// mechanism — StoreAccess has no "list all containers of type X"
    /// today, only keyword search, which isn't a reliable way to enumerate
    /// every AMT container. Best-effort: a logging failure here must never
    /// affect the orchestration response itself.
    fn record_amt_reexpansion_candidate(container_id: u64, project_id: u64, unverified_content: &str) {
        // Unified expansion-candidate store (amt_candidates) — route:
        // UnverifiedNode, the original in-build trigger.
        let _ = crate::orchestrator::amt_candidates::append(
            container_id,
            Some(project_id),
            "UnverifiedNode",
            Some(unverified_content),
        );
    }

    /// First unverified node found via depth-first walk, if any — real
    /// provenance-based signal (AMTNode.verified is never a fabricated
    /// score, see its own doc comment), not a heuristic guess.
    fn find_unverified_node(node: &AMTNode) -> Option<&AMTNode> {
        if !node.verified {
            return Some(node);
        }
        for child in &node.children {
            if let Some(found) = Self::find_unverified_node(child) {
                return Some(found);
            }
        }
        None
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
                        self.record_thinking(state, "Build AMT — branch discovery", &result);
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
                .read()
                .expect("K-ALGORITHM convergence lock poisoned")
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
        // Previously hardcoded (10 / 5), completely disconnected from the
        // K-ALGORITHM convergence preset system (config.toml's
        // [k_algorithms] convergence_preset) — every real orchestration
        // this session used THIS path (Mode: ChunkZeroShot in every log),
        // never the sibling build_amt_from_graphs builder that already
        // respects the configurable preset, so the pass cap was never
        // actually under user control despite that system existing. Now
        // reads the same live preset both builders should agree on.
        let max_outer_passes = crate::k_registry::KAlgorithms::global()
            .convergence
            .read()
            .expect("K-ALGORITHM convergence lock poisoned")
            .default_preset()
            .max_passes;
        // Tied to the same value rather than a separate hardcoded 5: with the
        // old pair (10 / 5), this early-exit heuristic was what actually
        // terminated every real run this session (~6 passes observed, well
        // under the unreachable hardcoded ceiling of 10). Left at a fixed 5
        // while max_outer_passes became configurable, a "fast" preset
        // (max_passes=2) would make this heuristic permanently dead code
        // (can't accumulate 5 no-new passes in a 2-pass run) while a
        // lower-than-5 custom preset would silently win over the user's
        // explicit ceiling. Matching it to max_outer_passes makes the
        // configured ceiling the sole effective governor, consistent with
        // how the sibling build_amt_from_graphs convergence check works.
        let convergence_threshold = max_outer_passes;
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

            // LEGACY PARITY: K (knowledge enrichment) + M (modal synthesis) —
            // the same contracts the graph-native builder runs per pass, so
            // both AMT modes see identical methodology/modal context.
            let layer_input = self.gather_layer_input(state);
            let knowledge = self.enrich_with_zsei_knowledge(state, &layer_input).await;
            let synthesis = self.synthesize_modal_evidence(&layer_input);
            let methodology_summaries_block = if knowledge.methodology_summaries.is_empty() {
                String::new()
            } else {
                format!(
                    "APPLIED METHODOLOGY CONTEXT:\n{}\n",
                    knowledge.methodology_summaries.join("\n")
                )
            };
            let cross_modal_block = if synthesis.cross_modal_summary.is_empty() {
                String::new()
            } else {
                format!(
                    "CROSS-MODAL STRUCTURE: {}\n",
                    synthesis.cross_modal_summary
                )
            };

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
{cross_modal_block}{methodology_summaries_block}
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
                    detected_modality_names.join(", "),
                    cross_modal_block = cross_modal_block,
                    methodology_summaries_block = methodology_summaries_block,
                );

                let intent_input = serde_json::json!({
                    "prompt": intent_prompt,
                    "max_tokens": 500,
                    "temperature": 0.2,
                    "system_context": "Extract new intents not already listed. Return only valid JSON. No explanation."
                });

                if let Ok(result) = self.metered_execute(state, 9, intent_input).await {
                    self.record_thinking(state, "Build AMT — intent extraction", &result);
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
                        self.record_thinking(state, "Build AMT — branch refinement", &result);
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

                                // No per-intent count limit, by explicit
                                // direction — the real control is
                                // find_methodologies_by_keywords' relevance
                                // bar (zsei/query.rs): a project genuinely
                                // touching many real concerns should get
                                // exactly as many real branches as it needs,
                                // not an arbitrary ceiling.
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
                                } else if already_exists {
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
                    self.record_thinking(state, "Build AMT — detail extraction", &result);
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

                                // No per-intent count limit — same reasoning
                                // as the methodology-driven branch path above.
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
        let pairwise = crate::k_registry::KAlgorithms::global()
            .pairwise
            .read()
            .expect("K-ALGORITHM pairwise lock poisoned")
            .default_preset()
            .clone();
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
                    self.record_thinking(state, "Build AMT — cross-reference", &result);
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

    /// Load a methodology's real decision_rules/heuristics content off disk
    /// (referenced by the container's storage.object_store_path, the same
    /// convention text/code modality graphs use) and format it as short,
    /// practical "IF condition THEN action" / "WHEN ... : ..." lines suitable
    /// for direct injection into an LLM prompt. Returns None when no content
    /// file exists yet (the container is keyword/name-only) rather than
    /// fabricating rule content — most of the 15 bootstrap methodologies are
    /// still in that state; only a few have real content written so far.
    pub(crate) fn load_methodology_rules_text(container: &serde_json::Value) -> Option<String> {
        let object_store_path = container
            .get("local_state")
            .and_then(|ls| ls.get("storage"))
            .and_then(|s| s.get("object_store_path"))
            .and_then(|p| p.as_str())?;

        let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
        // Absolute object_store_path used as-is; only relative paths join
        // the data dir — the third real occurrence of this exact bug class
        // tonight (amt_loop.rs's try_reexpand_one, jurisdiction.rs's
        // load_jurisdiction_rules, now here), found by task 62's own test
        // using an absolute temp-dir fixture path. Every prior real call
        // site here happened to pass a relative path, so this was latent,
        // not yet observed live.
        let full_path = if std::path::Path::new(object_store_path).is_absolute() {
            object_store_path.to_string()
        } else {
            format!("{}/{}", data_dir, object_store_path)
        };
        let content = std::fs::read_to_string(&full_path).ok()?;
        let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;

        let mut lines: Vec<String> = Vec::new();

        if let Some(rules) = parsed.get("decision_rules").and_then(|r| r.as_array()) {
            for rule in rules.iter().take(5) {
                let condition = rule.get("condition").and_then(|c| c.as_str()).unwrap_or("");
                let action = rule.get("action").and_then(|a| a.as_str()).unwrap_or("");
                if !condition.is_empty() && !action.is_empty() {
                    lines.push(format!("IF {} THEN {}", condition, action));
                }
            }
        }

        if let Some(heuristics) = parsed.get("heuristics").and_then(|h| h.as_array()) {
            for h in heuristics.iter().take(3) {
                let when = h.get("when_to_apply").and_then(|w| w.as_str()).unwrap_or("");
                let desc = h.get("description").and_then(|d| d.as_str()).unwrap_or("");
                if !when.is_empty() && !desc.is_empty() {
                    lines.push(format!("WHEN {}: {}", when, desc));
                }
            }
        }

        if lines.is_empty() {
            None
        } else {
            Some(lines.join(" / "))
        }
    }

    /// Record a real, observed methodology gap — a real request's keyword
    /// signal that matched zero methodologies — for the meta-loop
    /// (src/orchestrator/meta_loop.rs) to later review and, if it's a real
    /// recurring pattern rather than a one-off, draft and persist a new
    /// methodology for. Append-only JSON array, capped and deduplicated by
    /// near-identical keyword sets so a single busy request pattern doesn't
    /// flood the log. Best-effort: any I/O failure here is silently
    /// swallowed — this is an observability signal, not something that may
    /// ever affect the orchestration response itself.
    pub(crate) fn record_methodology_gap(keywords: &[String], topics: &[String]) {
        let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
        let path = format!("{}/methodology_gaps.json", data_dir);

        let mut gaps: Vec<serde_json::Value> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let mut sorted_keywords: Vec<String> = keywords.iter().map(|k| k.to_lowercase()).collect();
        sorted_keywords.sort();
        sorted_keywords.dedup();

        // Dedup against near-identical existing gaps (>=half the keywords
        // shared) rather than exact-match only — the same underlying gap
        // shows up with slightly different keyword extraction across runs.
        let already_recorded = gaps.iter().any(|g| {
            let existing: Vec<String> = g
                .get("keywords")
                .and_then(|k| serde_json::from_value(k.clone()).ok())
                .unwrap_or_default();
            if existing.is_empty() || sorted_keywords.is_empty() {
                return false;
            }
            let overlap = existing.iter().filter(|k| sorted_keywords.contains(k)).count();
            let smaller = existing.len().min(sorted_keywords.len());
            smaller > 0 && overlap * 2 >= smaller
        });

        if already_recorded {
            return;
        }

        gaps.push(serde_json::json!({
            "recorded_at": chrono::Utc::now().timestamp(),
            "keywords": sorted_keywords,
            "topics": topics,
            "handled": false,
        }));

        // Cap total log size — a true last-resort bound on disk usage for
        // an observability log, not a limit on methodology count itself.
        const MAX_GAP_LOG_ENTRIES: usize = 500;
        if gaps.len() > MAX_GAP_LOG_ENTRIES {
            let drop = gaps.len() - MAX_GAP_LOG_ENTRIES;
            gaps.drain(0..drop);
        }

        if let Ok(json) = serde_json::to_string_pretty(&gaps) {
            let _ = std::fs::write(&path, json);
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

            // Gap signal for the real methodology meta-loop (see
            // src/orchestrator/meta_loop.rs): a real request with a
            // non-trivial keyword signal (>=3 keywords — avoids recording
            // noise from a two-word throwaway prompt) that matched NO
            // methodology at all is exactly the case the user asked to have
            // searched for and eventually filled. Only record on a true
            // zero-match, not "found some but few" — a sparse-but-nonzero
            // match is still a real match, not a gap. Best-effort: a
            // logging failure here must never affect the orchestration
            // response itself.
            if found_methods.is_empty() && search_keywords.len() >= 3 {
                Self::record_methodology_gap(&search_keywords, &layer_input.topics);
            }

            // No count limit, by explicit direction: the real fix lives in
            // find_methodologies_by_keywords itself (zsei/query.rs) — it now
            // requires genuine keyword overlap, not any single incidental
            // match, and returns results sorted by relevance. Every
            // methodology that clears that bar is real signal; a project
            // can legitimately touch as many real concerns as it actually
            // touches, and the methodology store is meant to grow without
            // bound over time (bootstrap's 15 are a starting seed, not a
            // ceiling — methodology_create can add more at any time). An
            // earlier version of this code capped this loop at a fixed
            // count as a blunt guardrail before the real relevance fix
            // existed; removed now that the search itself is trustworthy.
            for method_id in found_methods.into_iter() {
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

                        // Real rule content (decision_rules/heuristics), not
                        // just the name+keywords summary — confirmed live
                        // this was a total gap: every one of the 15
                        // bootstrap-seeded methodologies had an
                        // object_store_path pointing at a content file that
                        // was never actually written anywhere, so branch
                        // discovery only ever had a bare name + generic
                        // keyword list to work from (explaining why matched
                        // methodologies produced vague, generic branches
                        // rather than anything grounded in an actual rule).
                        // Falls back to the keyword-only summary when no
                        // content file exists yet (most methodologies still
                        // don't have one — this is a real, only partially
                        // filled gap, not a claim that all 15 now do).
                        match Self::load_methodology_rules_text(&container) {
                            Some(rules_text) => {
                                knowledge.methodology_summaries.push(
                                    format!("[{}] — {}", name, rules_text)
                                );
                            }
                            None => {
                                knowledge.methodology_summaries.push(
                                    format!("[{}] covers: {}", name, keywords_str)
                                );
                            }
                        }
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
                self.record_thinking(state, "Build AMT — required domains", &result);
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

                if let Ok(synth_result) = self.metered_execute(state, 9, synth_input).await {
                    self.record_thinking(state, "Methodology Synthesis", &synth_result);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{RefinementConfig, TaskManager, TaskQueueConfig};

    // Records the parent_id create_container was actually called with —
    // T-A2 asserts against this directly rather than trusting the call
    // merely succeeded (MockZSEI elsewhere in this crate's test suite
    // returns a fixed id regardless of parent, which can't catch a
    // hardcoded-0-parent regression).
    struct RecordingStore {
        last_parent_id: std::sync::Mutex<Option<u64>>,
        /// Optional fixture: (project_id, child_ids) — get_container returns
        /// a project container with these children and a Derived-typed JSON
        /// for each child id, driving the continuation path in tests.
        project_fixture: Option<(u64, Vec<u64>)>,
        /// The container JSON from the last create_container call.
        last_container: std::sync::Mutex<Option<serde_json::Value>>,
    }

    impl RecordingStore {
        fn new() -> Self {
            Self {
                last_parent_id: std::sync::Mutex::new(None),
                project_fixture: None,
                last_container: std::sync::Mutex::new(None),
            }
        }
        fn with_project(project_id: u64, child_ids: Vec<u64>) -> Self {
            Self {
                last_parent_id: std::sync::Mutex::new(None),
                project_fixture: Some((project_id, child_ids)),
                last_container: std::sync::Mutex::new(None),
            }
        }
    }

    #[async_trait::async_trait]
    impl StoreAccess for RecordingStore {
        async fn query(&self, _q: serde_json::Value) -> Result<serde_json::Value, String> {
            Ok(serde_json::json!({}))
        }
        async fn traverse(&self, _r: serde_json::Value) -> Result<serde_json::Value, String> {
            Ok(serde_json::json!({}))
        }
        async fn create_container(
            &self,
            parent_id: u64,
            c: serde_json::Value,
        ) -> Result<u64, String> {
            *self.last_parent_id.lock().unwrap() = Some(parent_id);
            *self.last_container.lock().unwrap() = Some(c);
            Ok(4242)
        }
        async fn update_container(&self, _id: u64, _u: serde_json::Value) -> Result<(), String> {
            Ok(())
        }
        async fn get_container(&self, id: u64) -> Result<Option<serde_json::Value>, String> {
            if let Some((pid, ref children)) = self.project_fixture {
                if id == pid {
                    return Ok(Some(serde_json::json!({
                        "global_state": { "child_ids": children, "child_count": children.len() },
                        "local_state": { "metadata": { "container_type": "Project" } }
                    })));
                }
                if children.contains(&id) {
                    return Ok(Some(serde_json::json!({
                        "local_state": { "metadata": { "container_type": "Derived" } }
                    })));
                }
            }
            Ok(None)
        }
        async fn search_by_keywords(
            &self,
            _k: &[String],
            _t: Option<&str>,
        ) -> Result<Vec<u64>, String> {
            Ok(vec![])
        }
        async fn get_categories(&self, _m: &str) -> Result<Vec<u64>, String> {
            Ok(vec![])
        }
    }

    struct NoopExecutor;
    #[async_trait::async_trait]
    impl PipelineExecutor for NoopExecutor {
        async fn execute(
            &self,
            _pipeline_id: u64,
            _input: serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            Ok(serde_json::json!({"response": "{}"}))
        }
        async fn pipeline_exists(&self, _pipeline_id: u64) -> bool {
            true
        }
    }

    fn root_amt() -> AMTNode {
        AMTNode {
            id: 0,
            node_type: AMTNodeType::Root,
            content: "root analysis".to_string(),
            source_chunk_indices: vec![],
            children: vec![],
            relationships: vec![],
            methodology_ids: vec![],
            metadata: Default::default(),
            depth: 0,
            verified: true,
            confidence: 1.0,
        }
    }

    fn test_request(project_id: Option<u64>) -> OrchestrationRequest {
        OrchestrationRequest {
            prompt: "test".to_string(),
            project_id,
            workspace_id: None,
            user_id: 1,
            device_id: 1,
            consciousness_enabled: false,
            token_budget: Some(1000),
            model_config: None,
            attached_files: Vec::new(),
            processing_path: ProcessingPathPref::default(),
            executor_model: ExecutorModelKind::default(),
            voice_input: None,
            available_models: Vec::new(),
            fallback_order: Vec::new(),
            fallback_free_only: false,
            meta_fallback_order: Vec::new(),
            meta_fallback_free_only: false,
        }
    }

    fn test_state(project_id: Option<u64>) -> OrchestrationState {
        let request = test_request(project_id);
        OrchestrationState {
            request: request.clone(),
            start_time: std::time::Instant::now(),
            stages: Vec::new(),
            thinking_log: Vec::new(),
            jurisdiction_gate_result: None,
            model_context_limit: 200_000,
            tokens_used_so_far: 0,
            raw_chunks: Vec::new(),
            file_graphs: HashMap::new(),
            attached_file_contents: HashMap::new(),
            classified_file_graphs: Vec::new(),
            chunk_graph_ids: Vec::new(),
            modality_graphs: HashMap::new(),
            graph_states: HashMap::new(),
            root_modality_list: RootModalityList::default(),
            initial_graphs_created: false,
            cross_modal_index_id: None,
            processed_chunks: Vec::new(),
            cleaned_prompt: String::new(),
            prompt_tokens: 0,
            keywords: vec!["test".to_string()],
            entities: Vec::new(),
            topics: vec!["testing".to_string()],
            methodologies: Vec::new(),
            categories: Vec::new(),
            categories_created: 0,
            amt: None,
            amt_container_id: None,
            amt_validated: false,
            validation_streak: 0,
            amt_build_mode: AmtBuildMode::ChunkZeroShot,
            needs_clarification: false,
            clarification_points: Vec::new(),
            intent_captures: Vec::new(),
            branch_captures: Vec::new(),
            detail_captures: Vec::new(),
            cross_refs: Vec::new(),
            amt_pass_count: 0,
            coverage_aspects: Vec::new(),
            blueprint_id: None,
            blueprint_steps: Vec::new(),
            orch_step_states: HashMap::new(),
            blueprints_created: 0,
            task_id: None,
            step_results: Vec::new(),
            final_response: None,
            step_contexts: HashMap::new(),
            step_outputs: HashMap::new(),
            gate_result: None,
            voice_identity: None,
            available_pipelines: Vec::new(),
        }
    }

    fn test_orchestrator(store: Arc<dyn StoreAccess>, tmp: &str) -> PromptOrchestrator {
        let task_config = TaskQueueConfig {
            consciousness_enabled: false,
            storage_path: tmp.to_string(),
            ..Default::default()
        };
        let refinement_config = RefinementConfig { enabled: false, ..Default::default() };
        let task_manager = Arc::new(tokio::sync::RwLock::new(
            TaskManager::new(task_config, refinement_config).unwrap(),
        ));
        PromptOrchestrator::new(
            Arc::new(NoopExecutor),
            store,
            task_manager,
            Arc::new(RwLock::new(None)),
            200_000,
            crate::config::JurisdictionConfig::default(),
        )
    }

    // T-A2: a request with a real project_id gets its AMT container parented
    // to that project — the hardcoded-parent_id-0 bug class (a real bug
    // found and fixed earlier this session in text/code modality's own
    // persist_graph_container) stays dead here too.
    #[tokio::test]
    async fn amt_container_parents_to_real_project_when_project_id_present() {
        let store = Arc::new(RecordingStore::new());
        let orchestrator = test_orchestrator(
            store.clone() as Arc<dyn StoreAccess>,
            "/tmp/test_amt_parenting_a",
        );
        let state = test_state(Some(777));
        let amt = root_amt();

        let container_id = orchestrator.persist_amt_container(&state, &amt).await.unwrap();

        assert_eq!(container_id, 4242);
        assert_eq!(
            *store.last_parent_id.lock().unwrap(),
            Some(777),
            "AMT container must parent to the real project_id, not 0"
        );
    }

    // Companion case: no project_id (a one-off chat request) falls back to
    // the documented root-parent default (0), not some other stray value.
    #[tokio::test]
    async fn amt_container_falls_back_to_root_parent_when_no_project_id() {
        let store = Arc::new(RecordingStore::new());
        let orchestrator = test_orchestrator(
            store.clone() as Arc<dyn StoreAccess>,
            "/tmp/test_amt_parenting_b",
        );
        let state = test_state(None);
        let amt = root_amt();

        orchestrator.persist_amt_container(&state, &amt).await.unwrap();

        assert_eq!(*store.last_parent_id.lock().unwrap(), Some(0));
    }

    // MAIN/FORK lifecycle (island model), sequential — both phases share
    // the process-global OZONE_ZSEI_DATA_DIR env, so they run in one test.
    #[tokio::test]
    async fn amt_persist_main_then_fork_lifecycle() {
        let base = format!("/tmp/amt_lifecycle_{}", std::process::id());

        // ── Phase 1: first generation = MAIN ──
        let dir_main = format!("{}/main", base);
        std::fs::create_dir_all(format!("{}/amt", dir_main)).unwrap();
        std::env::set_var("OZONE_ZSEI_DATA_DIR", &dir_main);

        let store = Arc::new(RecordingStore::with_project(778, vec![]));
        let orchestrator = test_orchestrator(
            store.clone() as Arc<dyn StoreAccess>,
            &format!("{}/tasks", dir_main),
        );
        let state = test_state(Some(778));
        orchestrator
            .persist_amt_container(&state, &root_amt())
            .await
            .unwrap();

        let dir = format!("{}/amt", dir_main);
        let mut entries: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.path().to_string_lossy().into_owned())
            .collect();
        entries.sort();
        if entries.is_empty() {
            let listing = std::fs::read_dir(&dir)
                .map(|rd| rd.flatten().map(|e| e.path().to_string_lossy().into_owned()).collect::<Vec<_>>())
                .unwrap_or_else(|e| vec![format!("read_dir failed: {}", e)]);
            let env_val = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_default();
            panic!(
                "main AMT file written: dir={} env={:?} listing={:?}",
                dir, env_val, listing
            );
        }
        // MAIN/FORK keywords live on the CONTAINER (captured by the store),
        // not on the tree file — the tree file carries the Continues
        // relation instead.
        let captured = store
            .last_container
            .lock()
            .unwrap()
            .clone()
            .expect("create_container captured the AMT container");
        let kws = captured["local_state"]["context"]["keywords"]
            .as_array()
            .expect("keywords array");
        assert!(kws.iter().any(|k| k == "amt-main"), "first generation is MAIN");

        // ── Phase 2: subsequent generation = FORK of the prior ──
        let dir_fork = format!("{}/fork", base);
        std::fs::create_dir_all(format!("{}/amt", dir_fork)).unwrap();
        std::env::set_var("OZONE_ZSEI_DATA_DIR", &dir_fork);

        let store = Arc::new(RecordingStore::with_project(777, vec![500]));
        let orchestrator = test_orchestrator(
            store.clone() as Arc<dyn StoreAccess>,
            &format!("{}/tasks", dir_fork),
        );
        let state = test_state(Some(777));

        let container_id = orchestrator
            .persist_amt_container(&state, &root_amt())
            .await
            .unwrap();
        assert_eq!(container_id, 4242);

        let dir = format!("{}/amt", dir_fork);
        let mut entries: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.path().to_string_lossy().into_owned())
            .collect();
        entries.sort();
        assert!(!entries.is_empty(), "fork AMT file written");
        let fork_amt: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(entries.last().unwrap()).unwrap())
                .unwrap();
        let rel = fork_amt["relationships"].as_array().unwrap();
        assert_eq!(rel.len(), 1, "Continues relation present");
        assert_eq!(rel[0]["relation_type"], "Continues");
        assert_eq!(rel[0]["target_id"], 500);
        // Fork provenance keyword lives on the CONTAINER (captured).
        let captured = store
            .last_container
            .lock()
            .unwrap()
            .clone()
            .expect("fork container captured");
        let kws = captured["local_state"]["context"]["keywords"]
            .as_array()
            .unwrap();
        assert!(kws
            .iter()
            .any(|k| k.as_str().unwrap_or("").starts_with("amt-fork-of:")));
    }
}