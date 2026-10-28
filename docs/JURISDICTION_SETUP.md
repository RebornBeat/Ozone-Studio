# Jurisdiction-Aware Guardrails — Setup

> **UPDATE 2026-10-28:** national content NOW ships (131 national/local files, 648 rules, UN layer expanded) — the sentences below saying "zero content for any national or local jurisdiction" are history. See "Coverage snapshot" and "How to expand or refresh jurisdiction content" at the end of this file.
>
> **STATUS CORRECTION 2026-10-23 (read before acting on this document):**
> parts of this guide are stale relative to the tree. Verified current facts:
> the I-Loop EXISTS and runs (`src/orchestrator/i_loop.rs`, boot-spawned,
> live on the local + EU hosts); the ripple actors EXIST
> (`src/orchestrator/actors.rs` — SecurityFinding / CoordinationFinding /
> TaskFailed / CodeStaleness, boot-registered) and a sensor tick now feeds
> them; the findings dedupe key is FIXED (finding-routes dedupe on
> container+route+detail-digest — global findings no longer drop each other);
> the P2P trust gate is BUILT and BIDIRECTIONAL in the tree (paired_peers.json,
> default-deny both directions) with a peer-sync review pass surfacing
> received signals as finding containers; the queue processor exists behind
> OZONE_AUTO_EXECUTE_TASKS=1 (approval-gated). Authoritative status:
> `CHECKLIST.md` (2026-10-23 entries) + `docs/STATE_CAPTURE.md`. This
> document's prose below is retained as design history, not current state.

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

## Elevating a rule from Log-only (the concrete how-to, 2026-10-23)

Rules ship with national content at action `"Log"` deliberately — elevation
is an operator policy act, never automatic. To elevate:

1. Edit the rule's `action` in its JSON file (`assets/jurisdiction/<file>`)
   to one of `Block` / `Warn` / `RequireConfirmation` (the three real
   enforcement actions; `Log` records only).
2. Re-seed the graph copy the same way the file was originally loaded
   (registration-time derivation re-runs at boot; an already-seeded
   JurisdictionRuleSet container needs its UpdateContainer to carry the new
   action — rules load from the GRAPH at stage 0, rules_source tells you
   which layer served).
3. Verify: the stage-0 log line reports `rules_loaded` + `rules_source`;
   the compiled global fallback fires (loudly) only when zero Global rules
   exist. Only 7 rules act today (2 global Block, 3 global
   RequireConfirmation, 2 Warn) — every elevation moves the real
   enforcement boundary, so review the matcher behavior for the rule's
   trigger words first (`contains_word` is word-boundary safe including
   `_`, but English-only).

## Coverage snapshot (2026-10-28 expansion)

Shipped content under `assets/jurisdiction/`: `global.json` (the UN-scope layer: 7 baseline rules + 34 UN-system instruments, all new ones `Log`) and **131 national/local files** (was 40) holding **648 rules in total** (was 161): all 27 EU member states (+ `eu.json` for EU-wide law), the UN-member countries of every region that had retrievable law, and 8 sub-national files (`us-ca us-co us-il us-ny us-tx us-va us-wa`, `ca-qc`; scope `{"Local":"US-CA"}`). 308 rules carry local-language `aliases`. Every shipped rule is `Log`, EXCEPT the operator's own elevations (DO `personal data`, EU `personal data` + `data breach` = RequireConfirmation) and the 7 baseline global rules. `SOURCE_REGISTRY.md/.json` lists every rule with its source, official flag, snippet flag and alias count (regenerate with `python3 tools/generate_jurisdiction_registry.py`).

Honest limits (read before relying on any of it):
- Content was gathered by live web search, not from training memory. Where the search tool returned only summaries, the rule's `retrieved_snippet` says "search summary (not fetched)". Many official gazette sites refused fetches (403 / JS-only), so a large share of rules cite a real URL whose page was NOT read in full. ~220 rules have `official_source` not true (law-firm trackers, news, unofficial translations). Re-verify before treating any rule as a primary citation.
- Coverage is uneven: ~60 scopes have only 1-2 rules (mostly `personal data` + `cybercrime`). Absence of a topic means "not verifiable this pass", not "no such law". Bills/drafts were deliberately not recorded as rules.
- Some laws were reported amended/replaced after the retrieved page (e.g. MX personal-data law replaced in 2025 — the pre-2025 `personal data`/`minor`/`child` citations in `mx.json` are stale; BD cyber law reportedly repealed by ordinance in 2025; Colorado AI Act effective date conflicts between sources). Each such rule says so in its snippet.
- Matching is still whole-word on a lowercased request. English `condition` + per-rule `aliases`; aliases in unsegmented scripts (Chinese/Japanese/Thai/Lao/Khmer/Myanmar/Tibetan) match as substrings. Aliases were only added where the term appeared in the retrieved law title/text.
- Nothing here is legal compliance. It is a place to hold cited rules and check them consistently.

## How to expand or refresh jurisdiction content (the research how-to)

1. Pick the scope. File = `assets/jurisdiction/national/<iso2 lowercase>.json` (stem = scope keyword; scope value uppercase `{"National":"XX"}`); sub-national = `us-ca.json` with `{"Local":"US-CA"}`; UN-level = append to `global.json` with scope `"Global"`. EU member states automatically get a relationship edge to `eu` (list: `JURISDICTION_EU_MEMBER_CODES` in `src/lib.rs`) — put EU-wide law in `eu.json`, member-state implementing law in the member file.
2. Research (never from memory). For each topic in the checklist (personal data, data breach, child/minor online protection, consumer, cybercrime, hate speech/defamation, terrorism, copyright, AML/sanctions/crypto, health data, elections, AI law, gambling, interception, platform liability) run a web search against official domains (gazette, parliament, ministry, regulator, EUR-Lex, UN treaty bodies), then FETCH the page and confirm the statute title, number and year and a sentence you can quote. If the page cannot be fetched say so in the snippet. A smaller true file beats a larger guessed one.
3. Write the rule: `{"scope":..., "condition":"<1-3 lowercase English words a prompt would contain>", "aliases":["<local-language terms taken from the retrieved law>"], "action":"Log", "source":"<law name, number, year> — <URL>", "retrieved_snippet":"<what the page actually said>", "official_source": true|false}`. `official_source` true only for an official government / parliament / gazette / treaty-body domain. Keep a non-empty `disclaimer` with the retrieval date. Unique `condition` per file. Do not record bills or unconfirmed amendments as rules.
4. Validate: every file parses; conditions lowercase and unique; `action` is `Log` for anything you add (elevation is the operator's act, below); regenerate the registry.
5. Deploy: (a) host with the 2026-10-27 build: new files are copied and registered at boot; changed files are upgraded automatically when the data-dir copy is unmodified since the last sync, otherwise set `OZONE_JURISDICTION_FORCE_SYNC=1` once (old copy kept as `<name>.bak`); (b) any build: `python3 tools/sync_jurisdiction_content.py` shows NEW / DIFFERS vs a data dir and `--apply [--overwrite]` copies (old copies saved as `.bak`). New scopes become rule sets only after a host restart; changed content in an existing rule set is picked up on the next request (rules re-read at stage 0). Check the stage-0 log line for `rules_loaded` / `rules_source`.
6. Elevate deliberately, per rule, in the section above ("Elevating a rule from Log-only"). `assets/jurisdiction/ELEVATION_CANDIDATES.md` lists suggestions (UN layer, and DO/EU mapped onto real rule conditions) — suggestions only, nothing is applied by shipping content. A `RequireConfirmation` match costs a model review and an unjudged review halts the walk as ReviewPending; prefer specific conditions over everyday words.
