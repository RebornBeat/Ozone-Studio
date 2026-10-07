//! CONTEXT BUDGET + CONTEXT RECORDS — the host-side foundation for the
//! context-object model (docs/CONTEXT_OBJECT_MODEL.md,
//! docs/CONTEXT_BUDGET_SERVICE.md).
//!
//! The model the operator directed: context is OBJECTS in the ZSEI graph, not
//! text carried through the system. A stage holds references, traverses for
//! what it lacks, and materializes a prompt view only at the moment a model
//! is called, sized for THAT model's window. This module is step 1 of the
//! migration (capture foundation — zero behavior change) plus the shared
//! assembler every later step materializes through:
//!
//! - [`estimate_tokens`] — ONE shared estimator (the len/4+1 form the walk
//!   and the AMT lane packer already use), so every budget in the host speaks
//!   the same unit.
//! - [`assemble`] — priority sections fill the budget in the caller's order;
//!   when the budget is exceeded, the LOWEST-priority content is trimmed
//!   first at a paragraph boundary, and every cut is recorded
//!   (no silent caps — operator directive A4). When everything fits, the
//!   output is byte-identical to the caller's sections joined (behavior
//!   identical until a budget is exceeded — the migration contract).
//! - [`ContextRecord`] + [`record_call`] — one JSONL line per real model
//!   call: model, window, what went in, whether it was usable, and every
//!   trim. This is the object that later answers "what did this response
//!   actually see?" and the graph-tracked metrics surface.
//!
//! Deliberately NOT here yet (later migration steps): ContextNeed/ContextRef
//! resolution against the graph, aggregate objects, tool retrieval by need
//! (that seam lives at the capability summary), per-model windows on every
//! call site.

use std::sync::atomic::{AtomicBool, Ordering};

// Per-model window registry lives at crate::model_windows (declared in lib.rs).
// Re-exported here so existing `crate::context_budget::model_windows` paths
// keep resolving without touching their callers.
pub use crate::model_windows;

/// The shared token estimator. Same formula the fallback walk's context-fit
/// pre-order and the AMT lane packer use — one unit everywhere.
pub fn estimate_tokens(text: &str) -> usize {
    text.len() / 4 + 1
}

/// One prompt section offered to [`assemble`]. `priority` decides who is
/// trimmed first when the budget is exceeded: HIGHER number = trimmed first
/// (0 = protected, request/plan outrank history outrank summaries).
#[derive(Debug, Clone)]
pub struct ContextSection {
    pub name: String,
    pub priority: u8,
    pub text: String,
}

impl ContextSection {
    pub fn new(name: &str, priority: u8, text: impl Into<String>) -> Self {
        Self { name: name.to_string(), priority, text: text.into() }
    }
}

/// A recorded cut. Emitted for every trim — the operator's "no silent caps"
/// rule: what was removed, from where, how much.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrimRecord {
    pub section: String,
    pub original_tokens: usize,
    pub kept_tokens: usize,
    /// true when the whole section was left out of the view.
    pub dropped_entirely: bool,
}

/// The materialized view [`assemble`] returns: the prompt text plus the
/// record of every input it included or excluded.
#[derive(Debug, Clone)]
pub struct AssembledContext {
    pub text: String,
    pub trims: Vec<TrimRecord>,
    pub estimated_tokens: usize,
}

/// Cut `text` to at most `max_chars` characters at the last paragraph
/// boundary that fits (char-boundary safe — never splits a multi-byte
/// character; the byte-slice panic class is fixed everywhere else, this
/// path must not reintroduce it). Falls back to a char-boundary-safe hard
/// cut when a single paragraph exceeds the whole budget.
pub fn trim_at_paragraph(text: &str, max_chars: usize) -> (String, bool) {
    if text.chars().count() <= max_chars {
        return (text.to_string(), false);
    }
    let clipped: String = text.chars().take(max_chars).collect();
    let boundary = clipped
        .rfind("\n\n")
        .map(|i| i + 2)
        .or_else(|| clipped.rfind('\n').map(|i| i + 1))
        .unwrap_or(0);
    let kept = if boundary >= max_chars / 4 {
        clipped[..boundary].to_string()
    } else {
        // Even one paragraph overruns the budget — hard cut on a char
        // boundary rather than fabricating a mid-word paragraph end.
        let mut end = max_chars;
        while end > 0 && !clipped.is_char_boundary(end) {
            end -= 1;
        }
        clipped[..end].to_string()
    };
    (kept, true)
}

/// Assemble sections into a view that fits `budget_tokens`. Caller's order is
/// preserved in the output (identity when everything fits); priority decides
/// the trim order (highest first). Every cut lands in `trims`.
pub fn assemble(sections: Vec<ContextSection>, budget_tokens: usize) -> AssembledContext {
    let mut kept: Vec<String> = sections.iter().map(|s| s.text.clone()).collect();
    let mut trims: Vec<TrimRecord> = Vec::new();
    // No implicit separators: the caller's section text owns its own
    // newlines, so an all-fits view is byte-identical to the caller's input.
    let running_tokens = |kept: &[String]| estimate_tokens(&kept.concat());

    if running_tokens(&kept) <= budget_tokens {
        let text = kept.concat();
        let estimated_tokens = estimate_tokens(&text);
        return AssembledContext { text, trims, estimated_tokens };
    }

    // Trim candidates: priority DESC (least important trimmed first); ties
    // keep caller order (stable sort). Each section is visited at most once.
    let mut order: Vec<usize> = (0..sections.len()).collect();
    order.sort_by(|&a, &b| sections[b].priority.cmp(&sections[a].priority));

    for &idx in &order {
        let current_tokens = running_tokens(&kept);
        if current_tokens <= budget_tokens {
            break;
        }
        let original_chars = kept[idx].chars().count();
        if original_chars == 0 {
            continue;
        }
        let over_tokens = current_tokens - budget_tokens;
        let over_chars = over_tokens.saturating_mul(4);
        let allow = original_chars.saturating_sub(over_chars);
        if allow == 0 {
            trims.push(TrimRecord {
                section: sections[idx].name.clone(),
                original_tokens: estimate_tokens(&sections[idx].text),
                kept_tokens: 0,
                dropped_entirely: true,
            });
            kept[idx] = String::new();
            continue;
        }
        let (kept_text, trimmed) = trim_at_paragraph(&kept[idx], allow);
        if trimmed {
            trims.push(TrimRecord {
                section: sections[idx].name.clone(),
                original_tokens: estimate_tokens(&sections[idx].text),
                kept_tokens: estimate_tokens(&kept_text),
                dropped_entirely: false,
            });
            kept[idx] = kept_text;
        }
        // Not trimmed → this section already fits its allowance; the next
        // candidate (or the final guard below) handles the remainder.
    }
    let text = kept.concat();
    let estimated_tokens = estimate_tokens(&text);
    if estimated_tokens > budget_tokens {
        // The budget could not be honored even after trimming every
        // trimmable section (single protected section larger than the
        // budget). Record honestly rather than silently exceeding.
        trims.push(TrimRecord {
            section: "(budget still exceeded after all trims)".to_string(),
            original_tokens: estimated_tokens,
            kept_tokens: budget_tokens,
            dropped_entirely: false,
        });
    }
    AssembledContext { text, trims, estimated_tokens }
}

/// One record per real model call — what the response actually saw.
/// Serialized as one JSONL line by [`record_call`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContextRecord {
    /// Unix millis, captured here so callers can't fabricate or forget it.
    pub ts_ms: u128,
    pub call_site: String,
    pub model: String,
    /// The window this record used (0 = unknown).
    pub window_tokens: usize,
    /// Where `window_tokens` came from: "registry" (looked up for the model
    /// that served the call), "configured" (the candidate's own configured
    /// window, used when the served model is not the configured one and no
    /// served-model window is known), or "unknown" (0, nothing to look up).
    pub window_source: &'static str,
    pub want_output_tokens: usize,
    /// Shared-estimator size of the prompt actually sent.
    pub prompt_tokens: usize,
    pub usable: bool,
    /// True when the served model's window is smaller than the input this
    /// call actually sent. Set after the call, never guessed before it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub served_window_exceeded: bool,
    /// Every cut made while materializing this view (empty when none).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub trims: Vec<TrimRecord>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl ContextRecord {
    pub fn new(call_site: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            ts_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
            call_site: call_site.into(),
            model: model.into(),
            window_tokens: 0,
            window_source: "unknown",
            want_output_tokens: 0,
            prompt_tokens: 0,
            usable: false,
            served_window_exceeded: false,
            trims: Vec::new(),
        }
    }
}

static SINK_DISABLED: AtomicBool = AtomicBool::new(false);

/// Append one ContextRecord to `zsei_data/capture/context_records.jsonl`
/// (relative to the host's working directory, consistent with the
/// model_calls/ capture precedent). BEST-EFFORT by contract: a capture
/// failure warns once and disables the sink — it must never block, panic,
/// or fail a model call. This is capture, not behavior.
pub fn record_call(record: ContextRecord) {
    if SINK_DISABLED.load(Ordering::Relaxed) {
        return;
    }
    use std::io::Write;
    let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let path = std::path::Path::new(&data_dir).join("capture/context_records.jsonl");
    let write = || -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
        let line = serde_json::to_string(&record)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        f.write_all(line.as_bytes())?;
        f.write_all(b"\n")
    };
    if let Err(e) = write() {
        SINK_DISABLED.store(true, Ordering::Relaxed);
        eprintln!(
            "context_records capture failed once, sink disabled (model calls unaffected): {e}"
        );
        return;
    }
    // TRIM MARKERS (docs/CONTEXT_OBJECT_MODEL.md: every cut is visible on the
    // graph, never silent): each trim in this record ripples as a graph
    // event, so the monitor/UI sees context cuts as first-class occurrences
    // alongside the JSONL capture. id 0 with the `context:` source is the
    // marker convention — nothing here points at a container.
    for t in &record.trims {
        crate::graph_events::emit(
            "context_trimmed",
            0,
            0,
            format!("trim:{}", t.section),
            "context",
            vec![format!(
                "kept {}/{} tokens{}",
                t.kept_tokens,
                t.original_tokens,
                if t.dropped_entirely { " (dropped entirely)" } else { "" }
            )],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimator_matches_the_walk_formula() {
        // The walk computes (p.len() + sys.len()) / 4 + 1 — this must stay
        // the same unit everywhere.
        assert_eq!(estimate_tokens(""), 1);
        assert_eq!(estimate_tokens("abcdefgh"), 3); // 8/4+1
    }

    #[test]
    fn identity_when_everything_fits() {
        let sections = vec![
            ContextSection::new("request", 0, "COUNT TO THREE"),
            ContextSection::new("history", 5, "old turns"),
        ];
        let out = assemble(sections, 10_000);
        assert_eq!(out.text, "COUNT TO THREEold turns");
        assert!(out.trims.is_empty());
    }

    #[test]
    fn highest_priority_trimmed_first_with_records() {
        let sections = vec![
            ContextSection::new("request", 0, "KEEP ".repeat(300).trim_end().to_string()),
            ContextSection::new("summaries", 9, "cut me ".repeat(400).trim_end().to_string()),
        ];
        let out = assemble(sections, 400);
        assert!(out.text.contains("KEEP"));
        assert!(!out.trims.is_empty(), "the cut must be recorded");
        assert_eq!(out.trims[0].section, "summaries");
        assert!(out.trims[0].kept_tokens < out.trims[0].original_tokens);
        assert!(!out.trims[0].dropped_entirely);
        assert!(out.estimated_tokens <= 400 + 1);
    }

    #[test]
    fn paragraph_boundary_preferred_and_char_safe() {
        let para = "abcde ".repeat(40); // one paragraph, 240 chars
        let text = format!("{}\n\n{}", para, "tail".repeat(200));
        let (kept, trimmed) = trim_at_paragraph(&text, 600);
        assert!(trimmed);
        assert!(kept.ends_with("\n\n"));
        assert!(kept.chars().count() <= 600);
        // Multi-byte safety: a cut inside 'é' must not panic.
        let mb = "é".repeat(100);
        let (kept_mb, _) = trim_at_paragraph(&mb, 10);
        assert!(kept_mb.chars().count() <= 10);
    }

    #[test]
    fn record_serializes_with_trim_fields() {
        let mut r = ContextRecord::new("test_site", "test-model");
        r.prompt_tokens = 42;
        r.usable = true;
        let line = serde_json::to_string(&r).unwrap();
        assert!(line.contains("\"prompt_tokens\":42"));
        assert!(!line.contains("trims"), "empty trims omitted");
    }
}
