//! CONTEXT NEEDS — the object-model resolution step
//! (docs/CONTEXT_OBJECT_MODEL.md steps 3–4).
//!
//! A stage does not carry text forward and does not get blobs injected: it
//! declares a [`ContextNeed`] (what it is about, which object kinds matter,
//! how deep to reach, what budget applies), resolution searches the living
//! graph for matching OBJECTS and returns [`ContextRef`]s — references, not
//! copies. A view is materialized only when a model is called
//! ([`materialize`]), sized for THAT call's budget through the shared
//! assembler, with every cut recorded and every exclusion stated.
//!
//! This module is deliberately store-agnostic via `StoreAccess` so pipelines
//! can share the contract later; the host's `Arc<dyn StoreAccess>` is the
//! first consumer.

use crate::context_budget::{assemble, estimate_tokens, AssembledContext, ContextSection};

/// What a stage needs from the graph. `kinds` holds ContainerType display
/// names ("Methodology", "Blueprint", "Insight", ...) — display names, not
/// enum variants, so pipelines and the UI can declare needs without
/// depending on host enums.
#[derive(Debug, Clone)]
pub struct ContextNeed {
    /// What this stage is about — the semantic anchor for keyword search.
    pub anchor_text: String,
    /// Extra search terms beyond the anchor's own words (entities, names).
    pub keywords: Vec<String>,
    /// Container-type display names to restrict resolution to; empty = any.
    pub kinds: Vec<String>,
    /// How many objects per kind, in search-relevance order.
    pub per_kind_limit: usize,
    /// Token budget for the materialized view.
    pub budget_tokens: usize,
}

impl ContextNeed {
    pub fn new(anchor_text: impl Into<String>, budget_tokens: usize) -> Self {
        Self {
            anchor_text: anchor_text.into(),
            keywords: Vec::new(),
            kinds: Vec::new(),
            per_kind_limit: 5,
            budget_tokens,
        }
    }

    /// Lowercased words (>2 chars) from the anchor plus the explicit
    /// keywords — the search vocabulary. Deduplicated.
    pub fn search_terms(&self) -> Vec<String> {
        let mut terms: Vec<String> = Vec::new();
        let mut push = |t: &str| {
            let t = t.trim().to_lowercase();
            if t.len() > 2 && !terms.contains(&t) {
                terms.push(t);
            }
        };
        self.anchor_text
            .split(|c: char| !c.is_alphanumeric())
            .for_each(&mut push);
        self.keywords.iter().for_each(|k| push(k));
        terms
    }
}

/// One resolved object: a REFERENCE (id + naming) with a short extract for
/// relevance judgment. The full object stays in the graph.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContextRef {
    pub container_id: u64,
    pub name: String,
    pub kind: String,
    /// The object's content extract at resolution time (never the whole
    /// object — materialization pulls more only if the budget allows).
    pub extract: String,
}

/// The materialized view: references, the text a model would see, and the
/// record of what was included or left out (nothing silent).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContextView {
    pub refs: Vec<ContextRef>,
    pub materialized: String,
    pub estimated_tokens: usize,
    pub trims: Vec<crate::context_budget::TrimRecord>,
    /// Objects found but NOT materialized (budget/relevance) — named, never
    /// silently gone: they remain in the graph and retrievable later.
    pub excluded: Vec<u64>,
    /// The need's budget, carried from resolution so materialize() stays
    /// need-free.
    pub budget: usize,
}

/// The store surface the resolver needs (a slice of StoreAccess, so the
/// resolver takes `&dyn` without depending on the full trait).
pub trait NeedStore: Send + Sync {
    fn search(
        &self,
        keywords: &[String],
        kind: Option<&str>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Vec<u64>> + std::marker::Send + '_>,
    >;
    fn load(
        &self,
        id: u64,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Option<serde_json::Value>,
                > + std::marker::Send
                + '_,
        >,
    >;
}

impl ContextNeed {
    /// Resolve the need against the store: search per kind (or kindless),
    /// load the top objects, build references. Objects that fail to load or
    /// carry no usable name/extract are counted as excluded, not dropped
    /// silently.
    pub async fn resolve(&self, store: &dyn NeedStore) -> ContextView {
        let terms = self.search_terms();
        let mut refs: Vec<ContextRef> = Vec::new();
        let mut excluded: Vec<u64> = Vec::new();
        let mut seen: std::collections::HashSet<u64> = std::collections::HashSet::new();

        if terms.is_empty() {
            return ContextView {
                refs,
                materialized: String::new(),
                estimated_tokens: 0,
                trims: Vec::new(),
                excluded,
                budget: self.budget_tokens,
            };
        }

        let kinds: Vec<Option<String>> = if self.kinds.is_empty() {
            vec![None]
        } else {
            self.kinds.iter().map(|k| Some(k.clone())).collect()
        };

        for kind in kinds {
            let ids = store.search(&terms, kind.as_deref()).await;
            for id in ids.into_iter().take(self.per_kind_limit) {
                if !seen.insert(id) {
                    continue;
                }
                match store.load(id).await {
                    Some(obj) => {
                        let name = obj
                            .get("local_state")
                            .and_then(|l| l.get("metadata"))
                            .and_then(|m| m.get("name"))
                            .and_then(|n| n.as_str())
                            .unwrap_or("")
                            .to_string();
                        let obj_kind = obj
                            .get("local_state")
                            .and_then(|l| l.get("metadata"))
                            .and_then(|m| m.get("container_type"))
                            .and_then(|t| t.as_str())
                            .map(String::from)
                            .or_else(|| kind.clone())
                            .unwrap_or_else(|| "Unknown".to_string());
                        // Extract: first paragraph-ish slice of the object's
                        // content/keywords — char-safe, bounded.
                        let raw = obj
                            .get("content")
                            .and_then(|c| c.as_str())
                            .or_else(|| {
                                obj.get("local_state")
                                    .and_then(|l| l.get("context"))
                                    .and_then(|c| c.get("keywords"))
                                    .and_then(|k| k.as_str())
                            })
                            .unwrap_or("");
                        let extract: String = raw.chars().take(400).collect();
                        if name.is_empty() && extract.is_empty() {
                            excluded.push(id);
                            continue;
                        }
                        refs.push(ContextRef {
                            container_id: id,
                            name,
                            kind: obj_kind,
                            extract,
                        });
                    }
                    None => excluded.push(id),
                }
            }
        }

        ContextView {
            materialized: String::new(),
            refs,
            estimated_tokens: 0,
            trims: Vec::new(),
            excluded,
            budget: self.budget_tokens,
        }
    }
}

/// Materialize a resolved view for a model call: one section per reference
/// (priority by relevance order — earlier refs are protected longer), header
/// states the total so exclusions are visible in the view itself. Byte-
/// identical to the joined refs when everything fits (the assembler's
/// identity contract).
pub fn materialize(view: &mut ContextView) {
    if view.refs.is_empty() {
        view.materialized = "(no matching objects in the graph for this need)".to_string();
        view.estimated_tokens = estimate_tokens(&view.materialized);
        return;
    }
    let total = view.refs.len();
    // The header is a protected section (priority 0) so the count line never
    // gets trimmed away, and the assembler accounts for its tokens.
    let mut sections: Vec<ContextSection> = vec![ContextSection::new(
        "header",
        0,
        format!("MATCHING OBJECTS ({} of {} found):\n", total, total),
    )];
    sections.extend(view.refs.iter().enumerate().map(|(i, r)| {
        // Later (less relevant) refs trim first: priority grows with i.
        ContextSection::new(
            &format!("{}#{}", r.kind, r.container_id),
            (i.min(u8::MAX as usize)) as u8,
            format!("- {} ({} #{}): {}", r.name, r.kind, r.container_id, r.extract),
        )
    }));
    let AssembledContext { text, trims, estimated_tokens } = assemble(sections, view.budget);
    view.materialized = text;
    view.trims = trims;
    view.estimated_tokens = estimated_tokens;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeStore {
        ids: Vec<u64>,
    }
    impl NeedStore for FakeStore {
        fn search(
            &self,
            _keywords: &[String],
            _kind: Option<&str>,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<u64>> + Send + '_>> {
            let ids = self.ids.clone();
            Box::pin(async move { ids })
        }
        fn load(
            &self,
            id: u64,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Option<serde_json::Value>> + Send + '_>,
        > {
            Box::pin(async move {
                if id == 7 {
                    return None; // load failure → excluded, not dropped
                }
                Some(serde_json::json!({
                    "local_state": {"metadata": {"name": format!("obj {id}"), "container_type": "Methodology"},
                                     "context": {"keywords": "testing"}},
                    "content": format!("content of {id} ")
                }))
            })
        }
    }

    #[tokio::test]
    async fn resolve_builds_refs_and_names_exclusions() {
        let need = ContextNeed::new("verify the testing pipeline end to end", 4_000);
        let store = FakeStore { ids: vec![1, 7, 2] };
        let view = need.resolve(&store).await;
        assert_eq!(view.refs.len(), 2, "objects 1 and 2 resolve");
        assert_eq!(view.excluded, vec![7], "load failure named as excluded");
        assert_eq!(view.refs[0].container_id, 1);
        assert!(view.refs[0].extract.contains("content of 1"));
    }

    #[test]
    fn materialize_identity_when_fits_and_header_counts() {
        let mut view = ContextView {
            refs: (0..3)
                .map(|i| ContextRef {
                    container_id: i,
                    name: format!("n{i}"),
                    kind: "Methodology".into(),
                    extract: format!("e{i} "),
                })
                .collect(),
            materialized: String::new(),
            estimated_tokens: 0,
            trims: Vec::new(),
            excluded: vec![],
            budget: 10_000,
        };
        materialize(&mut view);
        assert!(view.materialized.starts_with("MATCHING OBJECTS (3 of 3 found):"));
        assert!(view.materialized.contains("n0"));
        assert!(view.trims.is_empty());
    }

    #[test]
    fn search_terms_dedupe_and_ignore_short() {
        let mut need = ContextNeed::new("Verify the AMT, AMT pipeline!", 100);
        need.keywords = vec!["amt".into(), "blueprint".into()];
        let t = need.search_terms();
        assert_eq!(t, vec!["verify", "the", "amt", "pipeline", "blueprint"]);
    }
}
