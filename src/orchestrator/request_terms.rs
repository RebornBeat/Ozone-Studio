//! Deterministic request-term hygiene and the relevance-first methodology floor (2026-11-07).
//!
//! Evidence (live walks 1099 and 1148, a two-sentence explanation request): the model-extracted "keywords" were
//! `["you", "how the zzsuff… original works in two sentences"]` — a stopword and the WHOLE sentence. The keyword search found nothing, so
//! the Stage 3 methodology FLOOR loaded EVERY methodology in the store (87, then 99, growing with every methodology ever created); the AMT
//! applied all of them (61 branches), the blueprint planned 12 steps and invented 9 "missing capabilities". Plan size scaled with the
//! store, not the request. These pure functions fix both causes without dropping anything a real match would have found.

/// Function words / pronouns / fillers that are never useful search terms. Deliberately small and language-neutral in intent: content
/// verbs and nouns (explain, build, clock …) are NOT here.
const STOPWORDS: &[&str] = &[
    "a", "an", "the", "you", "your", "yours", "i", "me", "my", "mine", "we", "our", "us", "it", "its", "he", "she", "they", "them", "their",
    "is", "are", "was", "were", "be", "been", "being", "am", "do", "does", "did", "done", "has", "have", "had", "how", "what", "why", "when",
    "where", "which", "who", "whom", "whose", "this", "that", "these", "those", "to", "of", "in", "on", "at", "for", "with", "by", "from",
    "as", "and", "or", "but", "if", "then", "than", "so", "not", "no", "yes", "please", "can", "could", "would", "should", "will", "shall",
    "may", "might", "must", "just", "also", "about", "into", "over", "under", "implied", "there", "here", "very", "too", "some", "any",
];

fn is_stopword(w: &str) -> bool {
    STOPWORDS.contains(&w)
}

/// Lower-cased content words of `text`: alphanumeric runs (inner `-`/`_` kept) of length >= 3 that are not stopwords, de-duplicated in
/// first-seen order.
pub fn content_words(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in text.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_')) {
        let w = raw.trim_matches(|c| c == '-' || c == '_').to_lowercase();
        if w.chars().count() >= 3 && !is_stopword(&w) && !out.contains(&w) {
            out.push(w);
        }
    }
    out
}

/// Clean the extracted keywords and make sure the search has real terms:
/// * an item made only of stopwords ("you", "you (implied)") is dropped;
/// * a sentence-like item (more than 4 words — e.g. the whole prompt returned as one "keyword") is replaced by its content words;
/// * everything else is kept as the model gave it (lower-cased, trimmed, de-duplicated);
/// * only when fewer than 3 usable keywords remain (extraction failed or came back thin) are the prompt's own content words added.
///   A rich extraction is never widened.
pub fn sanitize_keywords(prompt: &str, extracted: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |k: String, out: &mut Vec<String>| {
        if !k.is_empty() && !out.contains(&k) {
            out.push(k);
        }
    };
    for kw in extracted {
        let cleaned = kw.split('(').next().unwrap_or("").trim().to_lowercase(); // "you (implied)" -> "you"
        let words: Vec<&str> = cleaned.split_whitespace().collect();
        if words.is_empty() || words.iter().all(|w| is_stopword(w)) {
            continue;
        }
        if words.len() > 4 {
            for w in content_words(&cleaned) {
                push(w, &mut out);
            }
        } else {
            push(cleaned, &mut out);
        }
    }
    if out.len() < 3 {
        for w in content_words(prompt) {
            push(w, &mut out);
        }
    }
    out
}

/// The Stage 3 methodology FLOOR (never an empty gather) when the keyword search found nothing. `candidates` = every methodology under the
/// methodology root as `(id, name, keywords)`. Order of preference: (1) those sharing a content word with the request (name words or
/// keywords); (2) the BASE orchestration methodology; (3) all of them (the previous behaviour, only for a store with neither).
pub fn floor_selection(request_words: &[String], candidates: &[(u64, String, Vec<String>)]) -> Vec<u64> {
    let relevant: Vec<u64> = candidates
        .iter()
        .filter(|(_, name, kws)| {
            let mut terms: Vec<String> = content_words(name);
            terms.extend(kws.iter().flat_map(|k| content_words(k)));
            terms.iter().any(|t| request_words.contains(t))
        })
        .map(|c| c.0)
        .collect();
    if !relevant.is_empty() {
        return relevant;
    }
    let base: Vec<u64> = candidates
        .iter()
        .filter(|(_, name, _)| name.to_lowercase().contains("base orchestration"))
        .map(|c| c.0)
        .collect();
    if !base.is_empty() {
        return base;
    }
    candidates.iter().map(|c| c.0).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_live_failure_is_repaired() {
        // exactly what walks 1099/1148 produced
        let got = sanitize_keywords(
            "Explain how zzsuffmw5qi3vg original works in two sentences.",
            vec!["you".to_string(), "how the zzsuffmw5qi3vg original works in two sentences".to_string()],
        );
        assert!(!got.iter().any(|k| k == "you"), "a stopword is never a keyword");
        assert!(got.contains(&"zzsuffmw5qi3vg".to_string()) && got.contains(&"original".to_string()), "the sentence became its content words: {got:?}");
        assert!(got.iter().all(|k| k.split_whitespace().count() <= 4), "no sentence-like keyword survives");
    }

    #[test]
    fn a_rich_extraction_is_not_widened_or_changed() {
        let kws = vec!["graph database".to_string(), "traversal".to_string(), "index".to_string(), "storage".to_string()];
        let got = sanitize_keywords("Explain how a graph database traversal works with an index and storage engine", kws.clone());
        assert_eq!(got, kws, "4 usable keywords => untouched");
    }

    #[test]
    fn implied_markers_and_duplicates_are_cleaned() {
        let got = sanitize_keywords("build a clock", vec!["you (implied)".into(), "Clock".into(), "clock".into()]);
        assert_eq!(got.iter().filter(|k| *k == "clock").count(), 1);
        assert!(!got.iter().any(|k| k.contains("implied") || k == "you"));
    }

    #[test]
    fn floor_prefers_relevant_then_base_then_all() {
        let c = vec![
            (1u64, "Dynamic Tracing".to_string(), vec!["tracing".to_string(), "debug".to_string()]),
            (2, "Clock Rendering".to_string(), vec!["clock".to_string(), "3d".to_string()]),
            (3, "Base Orchestration Methodology".to_string(), vec!["orchestration".to_string()]),
        ];
        assert_eq!(floor_selection(&content_words("please build a 3d clock"), &c), vec![2], "relevant wins");
        assert_eq!(floor_selection(&content_words("explain photosynthesis"), &c), vec![3], "base is the floor when nothing is relevant");
        let no_base = vec![(1u64, "A".to_string(), vec![]), (2, "B".to_string(), vec![])];
        assert_eq!(floor_selection(&content_words("anything"), &no_base), vec![1, 2], "no base either => all (never empty)");
        assert!(floor_selection(&[], &[]).is_empty(), "an empty store stays empty (the empty-store path creates the base methodology)");
    }
}
