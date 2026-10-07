//! Shared context assembler for pipeline crates (included with `#[path]`).
//!
//! Priority sections fill a token budget; when the budget is exceeded, the
//! lowest-priority content is trimmed first at a paragraph boundary, and every
//! cut is recorded as a [`TrimRecord`]. When everything fits, [`assemble`]
//! returns the caller's sections joined, byte-identical.
//!
//! SYNC DEBT: the host copy of this logic is `src/context_budget.rs`
//! (`estimate_tokens`, `ContextSection`, `TrimRecord`, `trim_at_paragraph`,
//! `assemble`). The two must be kept in step. The host copy also owns the
//! `ContextRecord` sink, which is host-only and is not part of this file.
//!
//! Unused items are expected: each pipeline includes this file and uses the
//! subset it needs.

#![allow(dead_code)]

/// The shared token estimator. Same formula as the host (`len / 4 + 1`), so a
/// budget computed in a pipeline and in the host speak the same unit.
pub fn estimate_tokens(text: &str) -> usize {
    text.len() / 4 + 1
}

/// One prompt section offered to [`assemble`]. HIGHER priority number means
/// trimmed first (0 = protected).
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

/// A recorded cut: what was removed, from which section, and how much.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrimRecord {
    pub section: String,
    pub original_tokens: usize,
    pub kept_tokens: usize,
    /// true when the whole section was left out of the view.
    pub dropped_entirely: bool,
}

/// The materialized view [`assemble`] returns.
#[derive(Debug, Clone)]
pub struct AssembledContext {
    pub text: String,
    pub trims: Vec<TrimRecord>,
    pub estimated_tokens: usize,
}

/// Cut `text` to at most `max_chars` characters at the last paragraph boundary
/// that fits. Char-boundary safe. Falls back to a char-boundary hard cut when a
/// single paragraph exceeds the whole allowance.
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
        let mut end = max_chars;
        while end > 0 && !clipped.is_char_boundary(end) {
            end -= 1;
        }
        clipped[..end].to_string()
    };
    (kept, true)
}

/// Assemble sections into a view that fits `budget_tokens`. The caller's order
/// is preserved; priority decides the trim order (highest first). Every cut is
/// recorded in `trims`.
pub fn assemble(sections: Vec<ContextSection>, budget_tokens: usize) -> AssembledContext {
    let mut kept: Vec<String> = sections.iter().map(|s| s.text.clone()).collect();
    let mut trims: Vec<TrimRecord> = Vec::new();
    let running_tokens = |kept: &[String]| estimate_tokens(&kept.concat());

    if running_tokens(&kept) <= budget_tokens {
        let text = kept.concat();
        let estimated_tokens = estimate_tokens(&text);
        return AssembledContext { text, trims, estimated_tokens };
    }

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
    }
    let text = kept.concat();
    let estimated_tokens = estimate_tokens(&text);
    if estimated_tokens > budget_tokens {
        trims.push(TrimRecord {
            section: "(budget still exceeded after all trims)".to_string(),
            original_tokens: estimated_tokens,
            kept_tokens: budget_tokens,
            dropped_entirely: false,
        });
    }
    AssembledContext { text, trims, estimated_tokens }
}
