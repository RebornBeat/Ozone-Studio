//! Scoped chat persistence (2026-11-02).
//!
//! Before this module the host kept NO conversation record: a chat transcript
//! lived only in the UI (browser storage), so the full response was never part
//! of the graph, and a restart or a second device lost it. Now every
//! `/orchestrate` turn — success, clarification, failure — is persisted as a
//! graph entity, scoped exactly like the request:
//!
//! ```text
//!   scope        parent container          session keyword
//!   global       0 (root)                  chatsession:global
//!   workspace    workspace_id              chatsession:ws:<id>
//!   project      project_id                chatsession:proj:<id>   (workspace kept as a tag)
//! ```
//!
//! * One `ChatSession` container per scope (found by its unique keyword, created
//!   on first turn). The global/meta chat and every project chat each get their
//!   own, so a chat container is itself scoped to global / workspace / project.
//! * One `ChatTurn` child per turn. Its content file holds the FULL prompt and
//!   the FULL response — no preview, no cut, no cap. (Log previews elsewhere are
//!   previews of this record, never a replacement for it.)
//! * The response text is then graphed through the text modality in the
//!   background, and the turn is linked (`RelatedTo`) to that graph and to the
//!   prompt graphs this run built, so the conversation is part of the living
//!   graph rather than a side file.
//!
//! Best-effort in the sense that a storage failure never fails the user's
//! answer — but every failure is logged loudly and the turn's content file is
//! written FIRST, so the full text survives even if graph creation fails.

use super::*;
use crate::types::container::ContainerType;

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Scope of one chat: derived from the request, identical for the UI chat
/// that sent it (global/meta chat = no project/workspace; project chat =
/// project id, workspace id as a tag).
#[derive(Debug, Clone)]
pub struct ChatScope {
    pub project_id: Option<u64>,
    pub workspace_id: Option<u64>,
}

impl ChatScope {
    /// Unique session keyword (one session container per value).
    pub fn session_keyword(&self) -> String {
        match (self.project_id, self.workspace_id) {
            (Some(p), _) => format!("chatsession:proj:{p}"),
            (None, Some(w)) => format!("chatsession:ws:{w}"),
            _ => "chatsession:global".to_string(),
        }
    }
    /// Scope keywords in the convention the ripple/actors already use (`proj:N`, `ws:N`, `scope:global` — the marker `ContainerScope::from_markers`
    /// and `graph_events` read; the plain word `global` was never recognised by either).
    pub fn scope_keywords(&self) -> Vec<String> {
        let mut kw = Vec::new();
        if let Some(p) = self.project_id {
            kw.push(format!("proj:{p}"));
        }
        if let Some(w) = self.workspace_id {
            kw.push(format!("ws:{w}"));
        }
        if self.project_id.is_none() && self.workspace_id.is_none() {
            kw.push("scope:global".to_string());
        }
        kw
    }
    fn dir_name(&self) -> String {
        match (self.project_id, self.workspace_id) {
            (Some(p), _) => format!("proj-{p}"),
            (None, Some(w)) => format!("ws-{w}"),
            _ => "global".to_string(),
        }
    }
    fn parent_id(&self) -> u64 {
        self.project_id.or(self.workspace_id).unwrap_or(0)
    }
}

fn build_chat_container(
    container_type: ContainerType,
    name: &str,
    path: String,
    keywords: Vec<String>,
    object_store_path: Option<String>,
    parent_id: u64,
) -> Result<serde_json::Value, String> {
    use crate::types::container::{
        CompressionType, Container, Context, GlobalState, IntegrityData, LocalState, Metadata,
        Modality, StoragePointers, TraversalHints,
    };
    let now = now_secs();
    let c = Container {
        global_state: GlobalState {
            container_id: 0,
            parent_id,
            child_ids: vec![],
            child_count: 0,
            version: 1,
        },
        local_state: LocalState {
            metadata: Metadata {
                container_type,
                modality: Modality::Text,
                created_at: now,
                updated_at: now,
                provenance: "orchestrator-chat".to_string(),
                permissions: 0,
                owner_id: 0,
                name: Some(name.to_string()),
                materialized_path: Some(path),
            },
            context: Context {
                categories: vec![],
                methodologies: vec![],
                keywords,
                topics: vec!["chat".to_string()],
                relationships: vec![],
                learned_associations: vec![],
                keyword_meta: Default::default(),
                scope: Default::default(),
                embedding: None,
            },
            storage: StoragePointers {
                db_shard_id: None,
                vector_index_ref: None,
                object_store_path,
                compression_type: CompressionType::None,
            },
            hints: TraversalHints::default(),
            integrity: IntegrityData::default(),
            file_context: None,
            code_context: None,
            text_context: None,
            external_ref: None,
        },
    };
    serde_json::to_value(c).map_err(|e| e.to_string())
}

/// Add a `RelatedTo` relation `from -> to` to `from`'s context (idempotent).
async fn link_related(store: &Arc<dyn StoreAccess>, from: u64, to: u64) -> Result<(), String> {
    let Some(container) = store.get_container(from).await? else {
        return Err(format!("container {from} not found"));
    };
    let mut ctx = container
        .get("local_state")
        .and_then(|l| l.get("context"))
        .cloned()
        .ok_or_else(|| "container has no context".to_string())?;
    let rels = ctx
        .get_mut("relationships")
        .and_then(|r| r.as_array_mut())
        .ok_or_else(|| "context has no relationships array".to_string())?;
    if rels.iter().any(|r| {
        r.get("target_id").and_then(|t| t.as_u64()) == Some(to)
            && r.get("relation_type").and_then(|t| t.as_str()) == Some("RelatedTo")
    }) {
        return Ok(());
    }
    rels.push(serde_json::json!({
        "target_id": to,
        "relation_type": "RelatedTo",
        "confidence": 1.0,
        "discovered_via": "ToolOutput",
        "graph_hops": null
    }));
    store
        .update_container(from, serde_json::json!({ "context": ctx }))
        .await
}

impl PromptOrchestrator {
    /// Find or create the `ChatSession` container for this scope.
    async fn chat_session_for(&self, scope: &ChatScope) -> Result<u64, String> {
        let kw = scope.session_keyword();
        if let Ok(ids) = self
            .store
            .search_by_keywords(&[kw.clone()], Some("ChatSession"))
            .await
        {
            if let Some(id) = ids.first() {
                return Ok(*id);
            }
        }
        let mut keywords = vec!["kind:chat".to_string(), "kind:chat_session".to_string(), kw.clone()];
        keywords.extend(scope.scope_keywords());
        let name = match (scope.project_id, scope.workspace_id) {
            (Some(p), _) => format!("Chat — project {p}"),
            (None, Some(w)) => format!("Chat — workspace {w}"),
            _ => "Chat — global".to_string(),
        };
        let path = format!("/chat/{}", scope.dir_name());
        let parent = scope.parent_id();
        let make = |parent: u64| {
            build_chat_container(
                ContainerType::ChatSession,
                &name,
                path.clone(),
                keywords.clone(),
                None,
                parent,
            )
        };
        match self.store.create_container(parent, make(parent)?).await {
            Ok(id) => Ok(id),
            // The scoped parent may not exist as a container (stale id): the
            // scope keywords keep it findable; fall back to the root.
            Err(e) if parent != 0 => {
                tracing::warn!(parent, error = %e, "chat session: scoped parent unavailable — creating under root, scope kept in keywords");
                self.store.create_container(0, make(0)?).await
            }
            Err(e) => Err(e),
        }
    }

    /// Persist one finished `/orchestrate` turn. Writes the FULL prompt and
    /// FULL response to the turn's content file FIRST, then creates the
    /// `ChatTurn` container, then (background) graphs the response text and
    /// links the turn into the graph. Returns the turn container id.
    pub(crate) async fn persist_chat_turn(
        &self,
        state: &OrchestrationState,
        response: &OrchestrationResponse,
    ) -> Option<u64> {
        let scope = ChatScope {
            project_id: state.request.project_id,
            workspace_id: state.request.workspace_id,
        };
        let session_id = match self.chat_session_for(&scope).await {
            Ok(id) => id,
            Err(e) => {
                tracing::warn!(error = %e, "chat persist: no session container — turn kept only in the content file");
                0
            }
        };

        let ts = chrono::Utc::now();
        let turn_key = format!(
            "{}-{}",
            response.task_id.map(|t| t.to_string()).unwrap_or_else(|| "notask".into()),
            ts.timestamp_nanos_opt().unwrap_or(0)
        );
        let rel_path = format!("chat/{}/{}.json", scope.dir_name(), turn_key);
        let record = serde_json::json!({
            "kind": "chat_turn",
            "ts": ts.to_rfc3339(),
            "scope": {
                "project_id": scope.project_id,
                "workspace_id": scope.workspace_id,
                "session_keyword": scope.session_keyword(),
            },
            "user_id": state.request.user_id,
            "device_id": state.request.device_id,
            "task_id": response.task_id,
            "blueprint_id": response.blueprint_id,
            "trace_id": response.trace_id,
            "success": response.success,
            "needs_clarification": response.needs_clarification,
            "clarification_points": response.clarification_points,
            "model_used": response.model_used,
            "total_tokens_used": response.total_tokens_used,
            "execution_time_ms": response.execution_time_ms,
            // FULL TEXT — never truncated.
            "prompt": state.request.prompt,
            "response": response.response,
            "error": response.error,
            "prompt_graph_ids": state.modality_graphs.values().cloned().collect::<Vec<u64>>(),
        });
        // Same base the readers of `object_store_path` use (OZONE_ZSEI_DATA_DIR, else the configured data dir).
        let base_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| self.data_dir.clone());
        let full_path = format!("{}/{}", base_dir, rel_path);
        if let Some(parent) = std::path::Path::new(&full_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match serde_json::to_vec_pretty(&record) {
            Ok(bytes) => {
                if let Err(e) = std::fs::write(&full_path, bytes) {
                    tracing::error!(path = %full_path, error = %e, "chat persist: content file write FAILED — full turn text not stored");
                    return None;
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "chat persist: serialize failed");
                return None;
            }
        }
        if session_id == 0 && scope.parent_id() != 0 {
            // No session container could be created; the content file stands.
        }

        let mut keywords = vec!["kind:chat".to_string(), "kind:chat_turn".to_string(), scope.session_keyword()];
        keywords.extend(scope.scope_keywords());
        if let Some(t) = response.task_id {
            keywords.push(format!("task:{t}"));
        }
        let name = format!(
            "Turn {}",
            response.task_id.map(|t| t.to_string()).unwrap_or_else(|| turn_key.clone())
        );
        let container = match build_chat_container(
            ContainerType::ChatTurn,
            &name,
            format!("/chat/{}/{}", scope.dir_name(), turn_key),
            keywords,
            Some(rel_path.clone()),
            session_id,
        ) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = %e, "chat persist: build turn container failed");
                return None;
            }
        };
        let turn_id = match self.store.create_container(session_id, container).await {
            Ok(id) => id,
            Err(e) => {
                tracing::error!(error = %e, content_file = %rel_path, "chat persist: turn container create failed (content file kept)");
                return None;
            }
        };

        // Link the turn to the prompt graphs this run built (they already
        // exist) — synchronous, cheap.
        for gid in state.modality_graphs.values() {
            if let Err(e) = link_related(&self.store, turn_id, *gid).await {
                tracing::warn!(turn_id, graph_id = *gid, error = %e, "chat persist: prompt-graph link failed");
            }
        }

        // Graph the FULL response text through the text modality (background:
        // entity/topic extraction can be slow, and must not delay the answer).
        if let Some(text) = response.response.clone().filter(|t| !t.trim().is_empty()) {
            let executor = self.executor.clone();
            let store = self.store.clone();
            let project_id = scope.project_id.unwrap_or(0);
            let content_path = full_path.clone();
            tokio::spawn(crate::orchestrator::traced(async move {
                let analysis = executor
                    .execute(
                        100,
                        serde_json::json!({
                            "action": {
                                "type": "Analyze",
                                "text": text,
                                "depth": "Standard",
                                "extract_entities": true,
                                "extract_topics": true,
                                "extract_structure": false
                            }
                        }),
                    )
                    .await;
                let analysis = match analysis {
                    Ok(a) => a,
                    Err(e) => {
                        tracing::warn!(turn_id, error = %e, "chat persist: response analysis failed — text remains stored in the turn's content file");
                        return;
                    }
                };
                let created = executor
                    .execute(
                        100,
                        serde_json::json!({
                            "action": {
                                "type": "CreateGraph",
                                "analysis_result": analysis.get("analysis").cloned().unwrap_or_default(),
                                "project_id": project_id,
                                "link_to_existing": false
                            }
                        }),
                    )
                    .await;
                let graph_id = created
                    .ok()
                    .and_then(|g| g.get("graph_id").and_then(|x| x.as_u64()));
                match graph_id {
                    Some(gid) => {
                        if let Err(e) = link_related(&store, turn_id, gid).await {
                            tracing::warn!(turn_id, graph_id = gid, error = %e, "chat persist: response-graph link failed");
                        }
                        // Record the graph id in the turn's content file (in place,
                        // like the AMT expansion does) so the file alone is complete.
                        if let Ok(raw) = std::fs::read_to_string(&content_path) {
                            if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&raw) {
                                v["response_graph_id"] = serde_json::json!(gid);
                                let _ = std::fs::write(
                                    &content_path,
                                    serde_json::to_vec_pretty(&v).unwrap_or_default(),
                                );
                            }
                        }
                    }
                    None => tracing::warn!(turn_id, "chat persist: response CreateGraph returned no graph_id"),
                }
            }));
        }
        Some(turn_id)
    }
}

impl PromptOrchestrator {
    /// The most recent turns of THIS chat scope (same scope the request came
    /// from: global / workspace / project) rendered as walk context, oldest
    /// first, FULL text of each turn (the step-context compaction pass handles
    /// any window overflow — nothing is pre-cut here). HOW MANY turns is sized
    /// by the READING model (operator 2026-11-06: model-agnostic, no fixed
    /// count): newest turns first until an eighth of the model's window
    /// (`state.model_context_limit / 8`) is used — the newest turn is always
    /// included whole. `OZONE_CHAT_CONTEXT_TURNS` is an optional explicit
    /// override (a count; 0 disables). Excludes the current request (it is
    /// persisted only after delivery). `None` when there is no history.
    pub(crate) fn recent_chat_context(&self, state: &OrchestrationState) -> Option<String> {
        let explicit: Option<usize> = std::env::var("OZONE_CHAT_CONTEXT_TURNS").ok().and_then(|v| v.parse().ok());
        if explicit == Some(0) {
            return None;
        }
        let scope = ChatScope {
            project_id: state.request.project_id,
            workspace_id: state.request.workspace_id,
        };
        let base = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| self.data_dir.clone());
        let dir = std::path::Path::new(&base).join("chat").join(scope.dir_name());
        let mut turns: Vec<serde_json::Value> = std::fs::read_dir(&dir)
            .ok()?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("json"))
            .filter_map(|p| std::fs::read_to_string(p).ok())
            .filter_map(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .collect();
        if turns.is_empty() {
            return None;
        }
        turns.sort_by(|a, b| {
            a.get("ts")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .cmp(b.get("ts").and_then(|t| t.as_str()).unwrap_or(""))
        });
        // newest-first until the model-sized budget is used (or the explicit count is reached); the newest turn is always whole
        let budget_tokens = (state.model_context_limit as usize / 8).max(256);
        let render = |t: &serde_json::Value| -> Option<String> {
            let prompt = t.get("prompt").and_then(|p| p.as_str()).unwrap_or("");
            let reply = t
                .get("response")
                .and_then(|r| r.as_str())
                .or_else(|| t.get("error").and_then(|e| e.as_str()))
                .unwrap_or("");
            if prompt.is_empty() && reply.is_empty() {
                return None;
            }
            Some(format!("User: {prompt}\nAssistant: {reply}\n\n"))
        };
        let mut picked: Vec<String> = Vec::new();
        let mut used = 0usize;
        for t in turns.iter().rev() {
            if let Some(max_turns) = explicit {
                if picked.len() >= max_turns {
                    break;
                }
            }
            let Some(block) = render(t) else { continue };
            let cost = block.len() / 4 + 1;
            if !picked.is_empty() && explicit.is_none() && used + cost > budget_tokens {
                break;
            }
            used += cost;
            picked.push(block);
        }
        picked.reverse(); // oldest first
        let out: String = picked.concat();
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scopes_are_distinct_and_project_wins() {
        let g = ChatScope { project_id: None, workspace_id: None };
        let w = ChatScope { project_id: None, workspace_id: Some(7) };
        let p = ChatScope { project_id: Some(3), workspace_id: Some(7) };
        assert_eq!(g.session_keyword(), "chatsession:global");
        assert_eq!(w.session_keyword(), "chatsession:ws:7");
        assert_eq!(p.session_keyword(), "chatsession:proj:3");
        assert_eq!(g.parent_id(), 0);
        assert_eq!(w.parent_id(), 7);
        assert_eq!(p.parent_id(), 3);
        assert_eq!(p.scope_keywords(), vec!["proj:3".to_string(), "ws:7".to_string()]);
        assert_eq!(g.scope_keywords(), vec!["scope:global".to_string()]);
    }

    #[test]
    fn turn_container_json_round_trips_into_container() {
        let v = build_chat_container(
            ContainerType::ChatTurn,
            "Turn 1",
            "/chat/global/1".to_string(),
            vec!["chat".to_string()],
            Some("chat/global/1.json".to_string()),
            0,
        )
        .unwrap();
        let c: crate::types::container::Container = serde_json::from_value(v).unwrap();
        assert_eq!(c.local_state.metadata.container_type, ContainerType::ChatTurn);
        assert_eq!(c.local_state.storage.object_store_path.as_deref(), Some("chat/global/1.json"));
    }
}
