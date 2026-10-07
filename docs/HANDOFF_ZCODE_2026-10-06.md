# Handoff to ZCode — 2026-10-06 (from Claude Code)

Purpose: ZCode takes over build, test, review, and capture. Claude Code has finished its edit rounds for this session and has NOT committed anything. Read this file first, then the CHECKLIST.md entries dated 2026-10-06 (cc), starting with "free-model failure policy", "build and test round", and "per-user allow_paid_models".

## Hard constraints (do not break)

- **Production host** is on 127.0.0.1:50051 (PID 179703 at handoff, cwd target/release, old binary from before this round). Do NOT stop, restart, or POST to it without the operator's go.
- **ZCode's own host** (gdb-attached, if still running) must not be killed without the operator's approval.
- **Test host**: use port 50052 with its own working directory and data directory, as Claude Code did. Recipe: copy `target/release/config.toml` to a scratch dir, set `[grpc] port = 50052`, set `[consciousness] enabled = false`, copy `target/release/zsei_data` into the scratch dir, and launch `target/release/ozone-studio` from there with `OZONE_ZSEI_DATA_DIR` pointing at the copy. Stop only the process whose cwd is that scratch dir.
- **No OpenRouter spend** without the operator's go: no live orchestrate, no `OZONE_PROBE_FREE_MODELS=1`, no free-model probe. The daily free quota is 1,000 requests. The probe is env-gated and off by default; keep it off.
- **Paid models** are a per-user setting (`[models] allow_paid_models`, default `false`). Do not hard-code a paid refusal, and do not change the default.
- The OpenRouter key is not in any shell environment we can read. Do not search for it or print it.

## What changed (uncommitted; `git diff --stat`: 54 files, +6026 / -1106)

New files (untracked): `src/model_ledger.rs`, `src/openrouter_quota.rs`, `src/model_windows.rs`, `src/context_budget.rs`, `src/mcp_graph.rs`, `src/orchestrator/lane_split.rs`, `src/pipeline/gate.rs`, `assets/pipelines/shared/context_assemble.rs`, `assets/pipelines/shared/semantic_relations.rs`.

Modified Rust (host): `src/lib.rs`, `src/config/mod.rs`, `src/grpc/mod.rs`, `src/orchestrator/mod.rs`, `src/orchestrator/stages.rs`, `src/orchestrator/amt.rs`, `src/orchestrator/amt_loop.rs`, `src/orchestrator/decision_review.rs`, `src/orchestrator/jurisdiction.rs`, `src/orchestrator/response.rs`, `src/consciousness/assistant.rs`, `src/integrity/mod.rs`, `src/pipeline/executor.rs`, `src/pipeline/mod.rs`, `src/pipeline/remote.rs`, `src/types/container.rs`, `src/zsei/mod.rs`, `src/zsei/query.rs`, `src/zsei/storage.rs`, `src/zsei/traversal.rs`.

Modified pipelines (24 files under `assets/pipelines/`): the modality crates (3D, BCI, CAD, IMU, biology, control, depth, electromagnetic, geospatial, haptic, hyperspectral, kinematics, network, proteomics, radar, sonar, sound, text, thermal), general (prompt, settings_tab, text_analysis, zero_shot_simulation), and consciousness/collective_consciousness.

## Built and tested (by Claude Code, this session)

- Release host `target/release/ozone-studio`: built at 19:23 after the last Rust edit. `cargo build --release --bin ozone-studio`, exit 0, no errors, 11m50s, single job.
- Library unit tests `cargo test --release --lib`: 114 passed, 0 failed.
- Pipeline fleet `scripts/build-pipelines.sh`: 38 built, 1 failed (`prompt`, three E0308 errors). Fixed: `run_bounded_child` now takes `&mut std::process::Command`. `prompt` rebuilt into the shared `target/pipelines` directory at 19:07.
- Test-host checks (no model calls, host reported zero Prompt pipeline runs): paid `api_model` refused with the reason while the flag is off; accepted with the flag on; `api_model = openrouter/free` accepted; `POST /orchestrate` with an empty prompt returns `attempt_trail: []` and `refusals: []`.

## NOT built or tested (review and test these)

- Live behaviour of the free-model walk: the trail, cause labels (`provider` / `model` / `local-runtime` / `ozone-studio`), the quota gate, the retry rule, and the ledger. Unit tests cover some of it. No live run has happened.
- The walk's `local_last` sort on the user path, and the pool placement.
- The response fields `attempt_trail` and `refusals` on the success path (only the error path was exercised).
- The settings-tab pipeline (`assets/pipelines/general/settings_tab/main.rs`): it still filters its own list to free and local models and does not read `allow_paid_models`.
- `src/zsei/storage.rs` F1–F7 write-behind rewrite and `src/zsei/traversal.rs` NodeSource per-hop reads: built, not exercised under load.
- The zero-shot simulation pipeline (rewritten to OpenAI chat format this session): not run against a live endpoint.
- `src/pipeline/executor.rs` bounded child kill (watchdog + 30 s): not exercised.
- The probe in `src/model_ledger.rs`: never run; keep it off.

## Known issues and open decisions (for the operator, and for ZCode to flag)

1. **231 containers with a global record but no local-state file** (examples: ids 30017–30020). The F4 fix in `storage.rs` now refuses them on load, so those methodologies drop out of registration. The old binary loaded them silently as empty. Same condition exists in production data. Decision needed: keep refusing (and find where the content went), or restore a loud-but-loading path.
2. **Settings-tab pipeline** does not honour `allow_paid_models`. Needs the flag passed in its input, or a host-side filter.
3. **BitNet in the user chain**: about 300 s per attempt, and it is the wall-time cost. A 5-minute end-to-end run only happens when free models answer. Operator decision: keep as last resort (current), or change.
4. **Attempt trail visibility**: trail reaches the response (`attempt_trail`), logs, and thinking log. Watchdog expiry is `ozone-studio`; a remote reqwest timeout is `provider`.
5. **`decision_review.rs`**: the primary identity default is not threaded on that path (`primary_model_identity(&model_input, None)`), so the duplicate guard cannot see the configured default there.
6. **`eprintln!` diagnostics** from the prompt pipeline may not reach the host log. The success JSON fields (`content_null`, `has_reasoning`, `completion_tokens`, `reasoning_tokens`, `max_tokens_sent`, `reasoning_field`) are the reliable path.
7. **Hangs not verified**: AMT loop pass (unbounded per pass); remote-pipeline callers were not traced for the new 120 s timeout; `stages.rs` stage 7 and 8 each take about 25.6 s (cause unknown).
8. **ModelLedger `record`** does a small blocking file append from async code. Fine at this volume; revisit if it shows in profiles.
9. **Strength-N vote loop** (`mod.rs` ~4026–4075) now records into the ledger; its vote logic is unchanged.
10. **Zero-shot `OZONE_API_MODEL` override** is not verified to be checked against `is_free`.
11. **Repository hygiene (operator decision)**: `.cargo/config.toml` is gitignored (lld linker, shared pipeline target); `Cargo.lock` has a change from the new `futures` dependency; `tools/bridges` is an orphan gitlink with dirty files; `zsei_data/` has 686 tracked files including runtime data (`zsei_data/amt/*.json`); the `ILoop` pipeline index entry (id 44) has no crate or binary.
12. **Root `config.toml`** `[models]` now points at the OpenRouter free router (was Anthropic). `target/release/config.toml` is the live file and was also set to `openrouter_pool = "free"`, `openrouter_pool_size = 3`.

## What ZCode should do, in order

1. Read this file and the CHECKLIST 2026-10-06 (cc) entries.
2. Re-run the host build and `cargo test --release --lib` against the current tree. If anything changed, rebuild the affected pipelines with `scripts/build-pipelines.sh`.
3. Review the NOT-built list above by reading the code. Report any compile or logic problem to the shared context with file:line.
4. Do the test-host checks in the recipe above. No live orchestrate and no probe without the operator's go.
5. Capture finalized edits: append to CHECKLIST.md (additively, never rewrite), and add a shared-context handoff note that names each changed file. Release any file claims you take.
6. Commit and push only if the operator asks. Nothing has been committed by Claude Code.
