//! Structured CONSCIOUSNESS_RESPONSE record (docs/VOICE_BOX_GUIDE.md §1,
//! docs/CONSCIOUSNESS_RESPONSE_INTEGRATION.md).
//!
//! The assistant's persisted insight used to carry its emotional coloring as
//! a free-text trailer (`[consciousness-response: ...]`) that downstream
//! consumers (chat, history, the voice pipeline) had to regex. This module
//! writes the SAME facts as a structured JSON record, one line per response,
//! in `$OZONE_ZSEI_DATA_DIR/consciousness/consciousness_responses.jsonl`,
//! so a consumer reads typed fields instead of parsing prose.
//!
//! Doctrine: every field is a real captured value or explicitly absent
//! (`null`). Nothing is defaulted to a plausible-looking number.

use std::io::Write;

/// Path of the append-only record file.
pub fn record_path() -> std::path::PathBuf {
    let dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    std::path::PathBuf::from(dir)
        .join("consciousness")
        .join("consciousness_responses.jsonl")
}

/// Inputs for one record. `emotion` fields come straight from
/// `get_current_emotional_state()`; callers pass what they read.
pub struct ResponseRecord<'a> {
    /// Producer, e.g. "assistant-check-up".
    pub source: &'a str,
    /// "text" | "voice" — the channel the response is destined for.
    pub channel: &'a str,
    /// The response body that was persisted (digest prose, not the trailer).
    pub content: &'a str,
    pub emotion: &'a str,
    pub valence: f32,
    pub arousal: f32,
    pub dominance: f32,
    /// Container id of the persisted insight (None if persistence failed).
    pub container_id: Option<u64>,
    /// How many findings fed the response.
    pub findings: usize,
    /// Trace correlation, when a trace was active.
    pub trace_id: Option<String>,
}

/// Build the JSON value (pure — unit-testable without I/O).
pub fn to_json(r: &ResponseRecord<'_>, at: u64) -> serde_json::Value {
    serde_json::json!({
        "type": "CONSCIOUSNESS_RESPONSE",
        "schema": 1,
        "at": at,
        "source": r.source,
        "channel": r.channel,
        "emotion": {
            "primary": r.emotion,
            "valence": r.valence,
            "arousal": r.arousal,
            "dominance": r.dominance,
        },
        "content": r.content,
        "container_id": r.container_id,
        "findings": r.findings,
        "trace_id": r.trace_id,
    })
}

/// Append one record. Returns Err on I/O failure — the caller logs it
/// loudly; a failed record never blocks the insight itself.
pub fn append(r: &ResponseRecord<'_>) -> std::io::Result<()> {
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = record_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    let line = serde_json::to_string(&to_json(r, at))
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    writeln!(f, "{}", line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_carries_typed_fields() {
        let r = ResponseRecord {
            source: "assistant-check-up",
            channel: "text",
            content: "two items overdue",
            emotion: "calm",
            valence: 0.5,
            arousal: 0.2,
            dominance: 0.4,
            container_id: Some(7),
            findings: 2,
            trace_id: None,
        };
        let v = to_json(&r, 100);
        assert_eq!(v["type"], "CONSCIOUSNESS_RESPONSE");
        assert_eq!(v["emotion"]["primary"], "calm");
        assert_eq!(v["container_id"], 7);
        assert!(v["trace_id"].is_null());
    }
}
