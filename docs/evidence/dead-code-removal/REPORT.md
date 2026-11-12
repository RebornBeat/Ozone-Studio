# Dead-code removal, UI demo key, structural cross-file call layer (2026-11-11)

Edit-only. Nothing compiled or run; rustfmt / `node --check` / `ts.transpileModule` parse-checks only. EU untouched.
The edits sit in commit 86145d9 (a concurrent commit swept the working tree); this report + plan §10c + CHECKLIST lines are the record.

## 1. Seven superseded items — removed (each verified: superseded, behaviour captured, nothing dropped)
| Item | Was | Superseder (verified by reading) | Capture check |
|---|---|---|---|
| `PromptOrchestrator::load_methodology_rules_text` (amt.rs) | body = `load_methodology_rules_limited(c, MAX, MAX)` | `load_methodology_rules_full` — byte-identical body | its doc comment moved onto `_full`; the 5 comment references (stages.rs, amt_loop.rs x3, methodology_create/main.rs) renamed to `_full`; zero code callers (grep src/assets/tests/tools) |
| `NetworkManager::on_item_announced` | one-line wrapper over `on_items_announced(vec![..])` | `on_items_announced` (AnnounceItem batch) | the hook NAME string `"on_item_announced"` is still registered and triggered inside `on_items_announced` (unchanged) |
| `publish_to_marketplace` | one-line wrapper over `publish_batch_to_marketplace(&[..])` | `publish_batch_to_marketplace` | the only real caller already used the batch form |
| `process_batch_sync` | drained `normal_queue` to paired peers | marketplace publish (one push); its periodic driver was already removed from lib.rs | nothing producing Normal items remains (see "left" below) |
| `on_methodology_created`, `on_blueprint_created`, `on_container_created` | per-container hook triggers, no caller since `spawn_network_hook_bridge` publishes manifests | `on_items_announced` / `AnnounceItem` (blueprints are skipped there by design) | their three default-hook registrations (log-only closures reachable only via the removed methods) removed with them; `on_item_announced` and `on_pipeline_completed` hooks kept |

Docs updated (live ones renamed/corrected; dated history — CHECKLIST, STATUS, dated reviews/audits — left as the record):
README, CONTEXT_REGISTRY (S6 + note), ZERO_SHOT_CALL_REGISTRY, TOP_DOWN_REVIEW_GUIDE, BUILDER_REGISTRY, KEYWORD_CAPTURE_AND_ML_GUIDE, P2P_AND_SHARED_MCP_COMMUNICATION_GUIDE (§2.2 + §5.6.2 corrected), developer/ZSEI_ARCHITECTURE_DEEP_DIVE, BUILD_AND_TEST_PLAN §10c.

### Left (not removed) — reasons
* `broadcast_high_priority`, `trigger_hooks` High/Normal branches, `high_priority_queue`/`normal_queue` fields and the pending counts in the status call: no remaining producer after this pass (only `trigger_hooks` callers with Never/Low priority remain) but they are the pub queue/status surface and were not in the listed seven — **for review**.
* `dry_run_first` (`types/integrity.rs:131`): NOT superseded — it is a field of the spec'd `RollbackRequest` (OZONE_STUDIO_SPECIFICATION §25.4, STORAGE_INTEGRITY_GUIDE) that `integrity::rollback()` never reads. An unimplemented feature, not dead code. Options: (a) wire it (`rollback` returns `analyze_impact` and applies nothing when true) — recommended; (b) remove with the spec edit.
* Other lists reviewed: guardrail-audit §4 F1–F3 (fixed numbers / `allow(dead_code)` fields) belong to the lifecycle/caps work and have no identified superseder; F4–F7 are live-by-policy; tests-in-code items other than `dry_run_first` (debug-binary fallback, console.debug, worktrees, tsc script) are not superseded code. Nothing else removed.

## 2. UI forged demo key — removed
`ui/src/services/store.ts authenticate()` derived a fake public key from the device id and a fake "signature" from it (and had no caller). Replaced: the desktop **main process** already holds the device's Ed25519 key and does the real challenge→signature→session exchange (`ensureSessionToken`, electron/main.js). New IPC `auth:ensure` (main.js, preload.js `auth.ensure`, typed in App.tsx) makes that real exchange and returns only `{success,error}`; the store sets `isAuthenticated` from it and never holds a token or a private key. No dev-build fallback fabricates auth. Browser/web mode keeps its existing paired-device Bearer token path (ozoneClient.ts) — unchanged.

## 3. Structural cross-file call layer (code pipeline, `assets/pipelines/modalities/code/main.rs`)
* Per-file graphs already carried external nodes `<module>.<symbol>` (`external:true`, `module`, `symbol`). New deterministic resolver `resolve_module_path` links them to the other file's own Function node by each language's path rules: **JS/TS** relative specifiers (+ extension / `index.*` candidates), **Python** relative (`.`/`..`) and dotted (sibling first, else a unique path suffix), **Rust** `crate::` / `self::` / `super::` / child-module paths (`name.rs` / `name/mod.rs`, crate root via the nearest `/src/`). Go imports are not parsed by the pipeline today → reported unresolved ("not defined for language").
* No confidence, no invented edges: external packages, ambiguous suffixes ("N files match"), a resolved file lacking the function (re-export), unanchored raw-text analyses are all recorded as `unresolved_reason`.
* Rust calls to imported names previously never got a `module` (the whole `use` path was the "module"); `rust_use_pairs` now derives (module, name) from `use a::b::name;` / `use a::b::{x, y};` (aliases/globs/nested groups skipped, not guessed).
* **Layer separation:** result is its own derived sidecar `graphs/code_xfile_<project_id>.json` (`"layer":"structural"`, written atomically). The per-file graph files/containers are never written by this layer; semantic enrichment never reads/writes the sidecar. A changed file replaces only its own index entry (keyed by normalised path); resolutions are re-derived wholesale (pure function of the index) ⇒ idempotent. `update_graph` untouched.
* Triggers: `CreateGraph` (project_id != 0) upserts that file's entry best-effort and returns a `cross_file` summary (files/resolved/unresolved) in the output; new action `ResolveCrossFileCalls {project_id}` rebuilds the index from the project's persisted graphs (repair; first pass for files linked before this layer). `assemble_dependency_graph` now sets `to_file` to the resolved project path when resolvable (raw module string otherwise, as before) — `ComputeReverseDependencies` still works (exact match first).
* Tests added (callgraph_tests, not run): JS/Python/Rust resolution incl. ambiguity and external, `rust_use_pairs`, end-to-end derive (resolved / missing symbol / external).
* Not done: an HTTP read endpoint/UI view for the sidecar (orchestrator/UI files are owned by other forks) — the file is in the data dir and returned in the pipeline output; wiring it into `/callgraph/files` is a small follow-up.

## Unverified
Everything is parse-only. Check at the first `cargo check` of the code pipeline: closure borrows in `resolve_module_path` (`exact`/`by_suffix`/`one`), `&segs[..]` slice types in the Rust arm, the `internal_imports` tuple clone of `ImportDef`, and that no other crate includes `extract_function_calls` with the old `_language` name (callers pass positionally). Removed network methods have no test or tool reference (grep).
