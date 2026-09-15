# Jurisdiction-Aware Guardrails — Setup

The jurisdiction gate (`src/orchestrator/jurisdiction.rs`) is a real, always-on
enforcement mechanism that runs on every orchestration request, independent
of `consciousness_enabled`. It ships with the Global (U.N.-scope) starting
content described below, and zero content for any national or local
jurisdiction — that must be sourced and loaded by a human familiar with the
actual applicable law, and reviewed by qualified legal counsel before being
relied on for real compliance.

**This mechanism does not, by itself, make any instance "compliant" with
anything.** It is a place to load real, cited rules and have them
consistently checked — no more, no less.

## What ships by default

`assets/jurisdiction/global.json` — a small, conservative starting set of
rules drafted from two stable, extremely well-known international
instruments (the Universal Declaration of Human Rights and the UN Convention
on the Rights of the Child), each with a specific article citation in its
`source` field. Every rule's `action` is a generic engineering policy (warn /
require human confirmation / block), never a specific legal conclusion. Read
the file's own `disclaimer` field before relying on any of it.

This file is copied into every fresh instance's data directory at boot
(`BootstrapManager::copy_jurisdiction_content`, same mechanism as
methodologies/blueprints) — but the **container** referencing it must be
created once per instance, since container creation needs a running ZSEI
instance that doesn't exist yet during the file-copy step of boot. Do this
once, after the instance's first successful boot:

```bash
curl -sS -X POST http://127.0.0.1:50051/zsei/query \
  -H 'Content-Type: application/json' \
  -d '{
    "query": {
      "CreateContainer": {
        "parent_id": 0,
        "container": {
          "global_state": {"container_id": 0, "child_count": 0, "version": 1, "parent_id": 0, "child_ids": []},
          "local_state": {
            "metadata": {
              "container_type": "JurisdictionRuleSet",
              "modality": "Unknown",
              "created_at": 0, "updated_at": 0,
              "provenance": "bootstrap", "permissions": 0, "owner_id": 0,
              "name": "Global Jurisdiction Rules (UDHR + CRC starting set)",
              "materialized_path": null
            },
            "context": {
              "categories": [], "methodologies": [],
              "keywords": ["global"],
              "topics": ["jurisdiction", "human-rights", "child-rights"],
              "relationships": [], "learned_associations": [], "embedding": null
            },
            "storage": {
              "db_shard_id": null, "vector_index_ref": null,
              "object_store_path": "jurisdiction/global.json",
              "compression_type": "None"
            },
            "hints": {"access_frequency": 0, "hotness_score": 0.0, "last_accessed": 0, "centroid": null, "ml_prediction_weight": 0.0},
            "integrity": {"content_hash": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], "semantic_fingerprint": [], "last_verified": 0, "integrity_score": 1.0, "version_history": []},
            "file_context": null, "code_context": null, "text_context": null, "external_ref": null
          }
        }
      }
    },
    "session_token": ""
  }'
```

`content_hash` must be exactly 32 bytes; `metadata.modality` is required
(use `"Unknown"` — jurisdiction rulesets aren't a content modality). The
timestamps above are placeholders; use real Unix timestamps in practice.

## Loading real national/local content later

1. Write a content file matching `JurisdictionContentFile`'s shape
   (`{"disclaimer": "...", "rules": [...]}`, matching
   `src/orchestrator/jurisdiction.rs`'s `JurisdictionRule` fields exactly) —
   sourced from an actual, current, authoritative legal reference for that
   jurisdiction, reviewed by qualified legal counsel. The `disclaimer` field
   is required; a file without one fails to load (by design).
2. Place it under `<data_dir>/jurisdiction/<name>.json`.
3. Create a `JurisdictionRuleSet` container the same way as above, with
   `keywords` including the region label you'll set as
   `instance_region` in `config.toml`'s `[jurisdiction]` section (e.g.
   `["us-ca"]` for California — lowercase, matching how
   `load_jurisdiction_rules` normalizes lookups), and
   `object_store_path` pointing at your new file.
4. Set `[jurisdiction] instance_region = "US-CA"` (or whatever) in
   `config.toml` and restart. Global-scope rules apply regardless of this
   setting; national/local rules only ever apply when a region is set here
   — either explicitly, or auto-detected from real hardware/OS signals
   (system timezone + locale; see `src/hardware_region.rs`) when left
   unset, since this is deliberately not a skippable manual setting. An
   explicit config value always wins over hardware detection. Hardware
   detection only fills this in when its two signals agree; when they
   disagree it stays unset (logged, both raw values shown) rather than
   guessing, since a wrong value here can BLOCK real requests — this is
   not IP geolocation and makes no network calls.
