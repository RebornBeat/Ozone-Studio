//! Monitor — the activity hub and agent feed for Ozone-Studio's monitoring
//! surface (dashboard panel, browser plugin, ZCode connector).
//!
//! Forked from the gamedev-all-in-one activity-hub concept, adapted to the
//! store-contract architecture: the hub is in-memory ring buffer + poll API
//! (SSE/streaming can layer on later without changing the event shape).
//!
//! Everything that connects — ZCode itself, browser plugin, pipelines,
//! engines — is an AGENT in the connect-model registry
//! (`pipeline::RemotePipelines`); this module records what they DO.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

pub const MAX_EVENTS: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActivityKind {
    Log,
    Agent,
    Tool,
    Job,
    Bridge,
    /// Browser-plugin / external-observer events.
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActivityLevel {
    Info,
    Ok,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEvent {
    pub id: u64,
    /// Unix seconds.
    pub timestamp: u64,
    pub kind: ActivityKind,
    pub level: ActivityLevel,
    pub source: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
}

#[derive(Default)]
pub struct ActivityHub {
    events: Mutex<Vec<ActivityEvent>>,
    next_id: AtomicU64,
}

impl ActivityHub {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an event (ring buffer, oldest dropped past MAX_EVENTS).
    pub fn record(
        &self,
        kind: ActivityKind,
        level: ActivityLevel,
        source: &str,
        message: impl Into<String>,
        detail: Option<serde_json::Value>,
    ) -> ActivityEvent {
        let event = ActivityEvent {
            id: self.next_id.fetch_add(1, Ordering::Relaxed),
            timestamp: now_secs(),
            kind,
            level,
            source: source.to_string(),
            message: message.into(),
            detail,
        };

        let mut events = self.events.lock().unwrap();
        events.push(event.clone());
        let overflow = events.len().saturating_sub(MAX_EVENTS);
        if overflow > 0 {
            events.drain(0..overflow);
        }
        event
    }

    /// Most recent events, newest first, optionally filtered by kind.
    pub fn recent(
        &self,
        limit: usize,
        kind: Option<ActivityKind>,
    ) -> Vec<ActivityEvent> {
        let events = self.events.lock().unwrap();
        let limit = limit.min(MAX_EVENTS);
        let filtered: Vec<ActivityEvent> = events
            .iter()
            .filter(|e| kind.as_ref().map(|k| e.kind == *k).unwrap_or(true))
            .rev()
            .take(limit)
            .cloned()
            .collect();
        filtered
    }

    pub fn len(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_recent_filter() {
        let hub = ActivityHub::new();
        hub.record(ActivityKind::Agent, ActivityLevel::Info, "test", "a", None);
        hub.record(ActivityKind::Tool, ActivityLevel::Ok, "test", "b", None);
        hub.record(ActivityKind::Agent, ActivityLevel::Warn, "test", "c", None);

        let all = hub.recent(10, None);
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].message, "c"); // newest first

        let agents = hub.recent(10, Some(ActivityKind::Agent));
        assert_eq!(agents.len(), 2);
        assert!(agents.iter().all(|e| e.kind == ActivityKind::Agent));
    }

    #[test]
    fn ring_buffer_caps() {
        let hub = ActivityHub::new();
        for i in 0..(MAX_EVENTS + 50) {
            hub.record(ActivityKind::Log, ActivityLevel::Info, "test", format!("{i}"), None);
        }
        assert_eq!(hub.len(), MAX_EVENTS);
        let recent = hub.recent(1, None);
        assert_eq!(recent[0].message, format!("{}", MAX_EVENTS + 49));
    }
}
