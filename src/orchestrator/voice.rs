//! //! Voice identity persistence through the store contract.

use super::*;

impl PromptOrchestrator {

    /// Persist the session's voice identity through the ZSEI store contract
    /// (StoreAccess trait — swappable backend). Search for the identity
    /// container first; update it, or create it on first sighting. Never
    /// transient-only: voice identity survives sessions.
    pub(crate) async fn persist_voice_identity(&self, voice: &VoiceIdentity) {
        let voice_json = match serde_json::to_value(voice) {
            Ok(v) => v,
            Err(_) => return,
        };
        let existing = self
            .store
            .search_by_keywords(
                &["voice".to_string(), "identity".to_string()],
                Some("VoiceIdentity"),
            )
            .await
            .unwrap_or_default();

        if let Some(&container_id) = existing.first() {
            // The identity lives in the container's content file (the typed
            // container has no slot for it); an `update_container` with a
            // `voice_identity` key was a silent no-op. Rewrite the file.
            let rel = self
                .store
                .get_container(container_id)
                .await
                .ok()
                .flatten()
                .and_then(|c| {
                    c.get("local_state")
                        .and_then(|l| l.get("storage"))
                        .and_then(|st| st.get("object_store_path"))
                        .and_then(|p| p.as_str().map(String::from))
                });
            match rel {
                Some(rel) => {
                    let base = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| self.data_dir.clone());
                    let abs = std::path::Path::new(&base).join(&rel);
                    let body = serde_json::json!({ "voice": voice_json });
                    if let Err(e) = std::fs::write(&abs, serde_json::to_vec_pretty(&body).unwrap_or_default()) {
                        tracing::warn!(path = %abs.display(), error = %e, "voice identity: content file rewrite failed");
                    }
                }
                None => tracing::warn!(container_id, "voice identity container has no content file — not updated"),
            }
        } else {
            let _ = self
                .store
                .create_container(
                    0,
                    serde_json::json!({
                        "container_type": "VoiceIdentity",
                        "metadata": {
                            "name": "voice_identity",
                            "description": "Persistent voice identity from consciousness self_model"
                        },
                        "context": {
                            "keywords": ["voice", "identity"]
                        },
                        "local_state": { "voice": voice_json }
                    }),
                )
                .await;
        }
    }
}