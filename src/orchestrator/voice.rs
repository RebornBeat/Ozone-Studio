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
            let _ = self
                .store
                .update_container(
                    container_id,
                    serde_json::json!({ "voice_identity": voice_json }),
                )
                .await;
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