# BUILD AND TEST PLAN — 2026-11-10 (fork P1, build-readiness pass; NOTHING was compiled or run to write this)

Everything below is prepared from reading the repo and the EU server's read-only state (`ssh`, `ls`, `systemctl cat`, checksums). No `cargo`, `npm`, `tsc` or test was run. Every claim of the form "will compile" is a **reading**, not a result — see "What could not be verified" in STATUS §V.

## 0. What is going to be built
* **Uncompiled range:** `93ee8ec` (last deployed build, EU binary 2026-11-10 06:44Z, lib 265/265, K-suite 9/0/1) → `HEAD` (`881d2ea` + this pass). In `src/`, `ui/src`, `assets/pipelines`, `shared/`: 44 files, ~4,970 insertions / ~300 deletions (the old figure "~10k lines" counts docs and blueprints too).
* **New Rust modules (all declared):** `orchestrator::{work_order, tool_creation, peer_import, offer_review}` are `pub mod` in `src/orchestrator/mod.rs:49,50,62,64`. No new top-level file lacks a declaration.
* **Pipeline crates that must be rebuilt** (everything else is byte-identical to the deployed source):
  1. `methodology_create` — Update can RETIRE rules (`remove_decision_rules` / `remove_heuristics`, `retire_rules`, `has_real_rule`); **required** for the refinement retirement (STATUS §P/§U).
  2. `traversal_ml` and 3. `text` — both compile `shared/contracts/keyword_model.rs` via `#[path]`, whose `cmp_v0` tie-break changed (fewer finds first). `methodology_create` compiles it too.
* **UI:** `ui/src` (new `WorkOrderTree`, `KeywordChips`, `MethodologyRefinementPanel`, reworked `MarketplacePanel`, client types). `package.json` `build` is `vite build` (no `tsc`), so a type error does not break the build — run `tsc --noEmit` separately (never done for these files).

## 1. Drift check (done; re-run before building)
Result of the check done for this plan: for every file changed since `93ee8ec` that still exists (69 non-doc files) plus the whole of `src ui/src shared proto tests assets/pipelines Cargo.toml Cargo.lock`, the EU tree `/opt/ozone-build` was compared to the repo by sha256.
* **Differences found (1 source file):** `src/bootstrap.rs` — the EU had the pre-`a2f405c` copy (44-methodology index, no methodology 34). **Fixed:** repo file rsynced to `/opt/ozone-build/src/bootstrap.rs` (sha256 now equal on both sides: `f509e646…509c`).
* **Not on the EU (by design):** `assets/blueprints/*.json` (19 files, repo-only data — they are not part of the build tree) and `CHECKLIST.md` / `tools/*` edits (EU `CHECKLIST.md`, `tools/ozone-shared-context/server.js`, `tools/keyword-measure/namespace_backfill.mjs` carry OLDER copies; none is built — sync them only if a tool run on the EU needs them).
* **EU-only (stray) files, reported and NOT deleted:** two runtime graphs `assets/pipelines/modalities/text/zsei_data/graphs/text_1794193729533599740.json`, `…text_1794282393564869448.json`; a differing `…/text/zsei_data/analysis_cache/text/5fc39aac….json` and `…/model_calls/pipeline_zero_shot_calls.jsonl` (runtime output of the text pipeline running on the EU). No stray edit of a source file exists.
* Re-run on the EU just before building (should print nothing):
```bash
# on the dev box, from the repo root
D="src ui/src shared proto tests Cargo.toml Cargo.lock assets/pipelines"
find $D -type f ! -path '*/node_modules/*' ! -path '*/target/*' ! -path '*/zsei_data/*' | sort | xargs sha256sum | sed 's/  / /' | sort -k2 > /tmp/l.sum
ssh root@161.97.114.105 "cd /opt/ozone-build && find $D -type f ! -path '*/node_modules/*' ! -path '*/target/*' ! -path '*/zsei_data/*' | sort | xargs sha256sum" | sed 's/  / /' | sort -k2 > /tmp/e.sum
diff /tmp/l.sum /tmp/e.sum
# if a source file differs: rsync the REPO copy to the EU (repo is truth); never the other way. Never use --delete-excluded
# (STATE_CAPTURE_2026-11-08 §16 recorded that mistake: it removed 83 pipeline Cargo.lock files).
rsync -a src/ root@161.97.114.105:/opt/ozone-build/src/      # (same for shared/ assets/pipelines/ with --exclude zsei_data --exclude target)
```

## 2. Pre-flight on the EU (read-only)
```bash
ssh root@161.97.114.105
export PATH=$HOME/.cargo/bin:$PATH            # cargo is NOT on a non-login ssh PATH (first attempt of 2026-11-03 built 0)
cd /opt/ozone-build
curl -s localhost:50051/health                 # healthy, active_tasks must be 0 before any swap
df -h /opt; free -m; nproc                     # at the time of writing: 141 GB free, 12 GB RAM (9.4 GB available), 6 cores, cargo 1.99.1
```
The live service runs `/opt/ozone-studio/ozone-studio` with `WorkingDirectory=/opt/ozone-studio`, data dir `zsei_data` (= `/opt/ozone-studio/zsei_data`), drop-ins `overrides.conf` (OZONE_TOOL_ROUTING=1, AUTO_EXECUTE_TASKS=1, PEER_DIALOGUE=1, REQUIRE_SESSION=1, MIN_AVAIL_MB=1536), `keyword-settle-off.conf` (OZONE_KEYWORD_SETTLE=off), `tools-dir.conf`.

## 3. Ordered commands (EU, in `/opt/ozone-build`)
| # | Command | Expected | Notes |
|---|---|---|---|
| 3.1 | `cargo check --release --bin ozone-studio 2>&1 \| tee /tmp/p1-check.log` | exit 0; warnings only | First compile of ~5k changed lines: **expect a few errors** (the previous uncompiled waves needed 1–12 mechanical fixes each — `CHECKLIST` L-entries for 2026-11-02 and `STATE_CAPTURE_2026-11-08 §16`). Fix on the REPO, re-sync, re-check. Warnings are not denied anywhere (`#![deny` grep: none). |
| 3.2 | `cargo build --release --bin ozone-studio 2>&1 \| tee /tmp/p1-build.log` | exit 0 | Previous EU host builds: 5m35s – 6m30s. The artifact is `target/release/ozone-studio` and is **not live** until §5 installs it. |
| 3.3 | `cargo test --release --lib 2>&1 \| tee /tmp/p1-test.log` | **expected 291 passed / 0 failed** = 265 deployed + 26 new `#[test]` (25 in `src/`, 1 in `shared/contracts`, which is part of the lib); a different total means a test was lost or a cfg is off — investigate, do not rationalise. | Local `cargo test --lib` was OOM-killed once (swap exhausted); the EU is the test box. Rows to see pass by name: `tool_creation::tests::*` (4), `keyword_program::tests::bookkeeping_tags_are_namespaced_*`, `has_tag_*`, `network::tests::*door*`/`split_chunks*`, `peer_import::tests::*` (7), `offer_review::tests::*` (3), `actors::tests::coordination_finding_actor_matches_the_namespaced_kind_too`, `jurisdiction::graph_tests::identical_warnings_*` + the UPDATED `categorize_*` (now 2 warnings), `chat_store` scope test (now `scope:global`), `context_mirror` (uses `has_tag`), `keyword_model::tests::found_but_never_used_ranks_below_never_found_*`, `amt::tests::rule_items_*`, `meta_loop::refinement_retire_tests::*`. |
| 3.4 | **Pipeline crates — tests first, then build** (`export T=/opt/ozone-build/target/pipelines`): `CARGO_TARGET_DIR=$T cargo test --release --manifest-path assets/pipelines/general/methodology_create/Cargo.toml` (expected ≈ 21 = the previous 13 + 7 new retire tests + 1 shared `keyword_model` test), then `…/general/traversal_ml/Cargo.toml` (≈ 14 = 13 + 1 shared), then `…/modalities/text/Cargo.toml` (≈ 40 + 3 = previous 39+3 + 1 shared). | all pass | The new test counts are derived by reading, not measured. |
| 3.5 | `CARGO_TARGET_DIR=$T cargo build --release --manifest-path assets/pipelines/general/methodology_create/Cargo.toml`, then `traversal_ml`, then `modalities/text`. **Not** `scripts/build-pipelines.sh` for this (it rebuilds the whole 50–60 fleet; nothing else changed). | exit 0 each | **IMPORTANT: this is live immediately.** `executor::builtin_candidates` bakes `CARGO_MANIFEST_DIR` (`/opt/ozone-build`) into the host and tries `<it>/target/pipelines/release/<name>` FIRST, so the RUNNING (old) host executes a pipeline binary the moment it is built (CHECKLIST 2026-11-03 recorded the same). For these three it is backward compatible by reading: `methodology_create` only adds optional fields to its output (`removed`, `previous_body_path`, skipped when `None`) and only acts on the new input fields; `text`/`traversal_ml` only change a rank tie-break. If you want zero exposure, build into another target dir (`CARGO_TARGET_DIR=/opt/ozone-build/target/p1-pipelines`) and copy the three binaries into `$T/release/` at the §5 swap. |
| 3.6 | UI (dev box): `cd ui && npx tsc --noEmit` then `npm run build` | tsc: unknown (never type-checked: expect a handful of type errors in the new components — `KeywordChips`, `WorkOrderTree`, `MethodologyRefinementPanel`, `MarketplacePanel`); build: should pass (esbuild only needs valid syntax + resolvable imports, all verified by reading) | `ui/package.json` `build` = `vite build`; tsc is not part of it. `noUnusedLocals` is false. |

If 3.1–3.3 fail: fix on the **repo**, rsync the touched file(s) to the EU, repeat from the failing step; record every fix in the STATUS section of the fix commit (the previous waves' practice). Do not edit on the EU first.

## 4. Backups and rollback (before §5)
```bash
TS=$(date -u +%Y%m%d%H%M%S)
cp -a /opt/ozone-studio/ozone-studio        /opt/ozone-studio/ozone-studio.bak-$TS      # naming used on this box: ozone-studio.bak-<UTC stamp>
cp -a /opt/ozone-studio/pipelines           /opt/ozone-studio/pipelines.bak-$TS         # (pipelines.bak-20261103231910, …20261109010624 exist)
cp -a /opt/ozone-studio/config.toml         /opt/ozone-studio/config.toml.bak-$TS
mkdir -p /opt/backups/p1-$TS && cp -a /etc/systemd/system/ozone-studio.service.d /opt/backups/p1-$TS/
```
**Rollback** (host stopped ~2 s, as in every earlier swap): `systemctl stop ozone-studio; cp -a /opt/ozone-studio/ozone-studio.bak-$TS /opt/ozone-studio/ozone-studio; cp -a /opt/ozone-studio/pipelines.bak-$TS/. /opt/ozone-studio/pipelines/; for n in methodology_create traversal_ml text; do cp -a /opt/ozone-studio/pipelines.bak-$TS/$n /opt/ozone-build/target/pipelines/release/$n; done; rm -f /etc/systemd/system/ozone-studio.service.d/p1-new-features-off.conf; systemctl daemon-reload; systemctl start ozone-studio`. (The shared-target copies must be restored too — they shadow `/opt/ozone-studio/pipelines`; see 3.5.) Data written meanwhile is additive (new files under `zsei_data`, `kind:`/`agent:` tag markers — readers of the OLD binary do not know `has_tag`, see §7 item 4).

## 5. Install and start (only with `active_tasks == 0`)
```bash
# 5.1 the deploy-OFF drop-in FIRST (see §6) so the new binary never runs a new feature before it is looked at
cat > /etc/systemd/system/ozone-studio.service.d/p1-new-features-off.conf <<'EOF'
[Service]
Environment=OZONE_TOOL_CREATION=off
Environment=OZONE_REFINEMENT_ACTION=off
Environment=OZONE_REFINEMENT_RETIRE=off
EOF
systemctl daemon-reload
# 5.2 swap
systemctl stop ozone-studio
cp /opt/ozone-build/target/release/ozone-studio /opt/ozone-studio/ozone-studio.new && mv /opt/ozone-studio/ozone-studio.new /opt/ozone-studio/ozone-studio
for n in methodology_create traversal_ml; do install -m 755 /opt/ozone-build/target/pipelines/release/$n /opt/ozone-studio/pipelines/$n; done
install -m 755 /opt/ozone-build/target/pipelines/release/text /opt/ozone-studio/pipelines/text
systemctl start ozone-studio; sleep 5
curl -s localhost:50051/health ; journalctl -u ozone-studio -n 80 --no-pager | grep -E "ERROR|panic|marketplace|P2P"
```
Expect healthy within ~2 s, **0 `ERROR`/panic**, and the line `P2P marketplace: disabled (marketplace_publish / marketplace_accept_offers / auto_broadcast_sync all false, default) — nothing is published or pulled`.

## 6. Deploy-OFF switches — exact semantics read from the code
| Switch | Where | Default (verified) | Deploy value | What it gates |
|---|---|---|---|---|
| `OZONE_TOOL_CREATION` | `tool_creation.rs:35-37` — `enabled() = !matches!(env, "off" \| "0" \| "false")` | **ON when unset or set to anything else** (including `1`, `on`, empty) | `off` | the E30 `capability_tool_draft` call site in the capability closure (one extra model call per necessary gap; writes `zsei_data/created_tools/<name>/` and a stdio registry row; host never runs it) |
| `OZONE_REFINEMENT_ACTION` | `meta_loop.rs:235` — off only for the exact value `off` (case-insensitive) | **ON when unset** | `off` | the meta-loop refinement VOTE + split (model calls; creates focused methodologies via pipeline 12 `Create`) |
| `OZONE_REFINEMENT_RETIRE` | `meta_loop.rs:408` — off only for `off` | **ON when unset** | `off` | retiring the moved rule from the original methodology (pipeline 12 `Update` with `remove_*`; needs the rebuilt `methodology_create`) |
| `OZONE_KEYWORD_SETTLE` | `keyword_loops.rs:82` (off for `off` / `0` / `false`) | already `off` on the EU (`keyword-settle-off.conf`) | keep `off` | keyword settlement passes |
| `marketplace_publish`, `marketplace_accept_offers` | `[network]` in `config.toml`, `config/mod.rs:608-653` | `false` (struct default AND `#[serde(default)]`; the EU `config.toml` has neither key nor `auto_broadcast_sync`, so both are false) | leave absent, or write `marketplace_publish = false` / `marketplace_accept_offers = false` explicitly | the whole marketplace: bridge, exchange, offer review, peer import. `auto_broadcast_sync = true` would turn BOTH on (deprecated alias) — must stay absent/false. `enable_p2p = true` on the EU is independent: the P2P layer stays up, the marketplace does not. |
| `OZONE_KEYWORD_REQUIRE_VALIDATED` | `keyword_program::require_validated_keywords` | OFF | leave unset | strict search (`strict_for_type` is a no-op while this is unset) |

**Behaviours of the new binary with NO kill switch** (they run as soon as the binary does — know them before the first walk): the rule gate (`judge_rule_applicability`, extra model calls per walk after the blueprint draft); work-order task records (`wo_*` tasks created per walk; hidden from the UI recent-task list); tag namespacing at the write chokepoint (every NEW write of `CoordinationEvent` / `Derived` run-span / `McpResult` / `McpEntity` / `ChatSession` / `ChatTurn` / `Experience` containers stores `kind:`/`agent:`/`span:`/`tool:`/`node:`/`flag:`/`outcome:` markers instead of plain words — nothing is migrated; `tools/keyword-measure/namespace_backfill.mjs` is a dry-run tool and was not applied); jurisdiction `Log` rules now surface as cited warnings; the quality audit now enumerates EVERY methodology (`list_ids`) and may raise `modify` gaps for the ~43 rule-less ones, which the (unchanged, un-switched) meta-loop gap pass will then try to draft; ranking tie-break (`cmp_v0`: fewer finds first); `GetContainerContent` falls back to the app root for bootstrap-seeded bodies.

## 7. First real walk — what to look at
Use the same prompt as before so the comparison is real: `WALK_PROMPT` default of `tools/e2e/walk-completion.mjs` ("Explain in two sentences why graph databases suit highly connected data."). The free-tier quota, not the code, decided the duration last time (Stage 2 4,227 s) — judge stages by their own timings, not by wall time.
1. **Rule gate** (`journalctl`): `rule gate complete (applicability + step fidelity)`, `blueprint gate: rule citations audited`, and Stage 6 "methodologies contributed rules". With one gathered methodology expect one round-trip per strength round (up to `configured_strength`), a failed call drops nothing.
2. **Work-order tree:** `curl -s localhost:50051/order/work/<task_id>` (task id from the walk's response) — walk root → `wo_branch` per AMT node → `wo_step` per blueprint step with a `run` item; items tick only as steps finish; a node's `complete` is derived, never set. `POST /task/list` rows carry `kind`, `parent_task_id`, `rerun_of`. UI: open the task → "Work order" block; the recent-tasks chip must NOT list `wo_*` rows.
3. **Capability closure with `OZONE_TOOL_CREATION=off`:** no `tool creation:` log line and no `zsei_data/created_tools/` folder; `capability_gaps.json` rows still get `methodology_drafted` / `not_needed` / `wanted`; `GET /capability/gaps` lists every row.
4. **Tag markers:** after the walk, `/zsei/query` a recent `CoordinationEvent` / chat turn: keywords begin with `kind:` / `agent:` (new writes only); the ripple actors still emit findings (`has_tag` accepts both forms) — check the security / coordination findings still appear.
5. **Marketplace stays dark:** `GET /marketplace/overview` → `exchange.publish_enabled=false`, `accept_offers_enabled=false`; `peer_marketplace_inbox.jsonl` not growing; nothing in `marketplace_published.json`.
6. **Refinement stays dark:** `GET /methodology/refinement` → `candidates` may grow (the rule gate records evidence) but `actions` stays empty while `OZONE_REFINEMENT_ACTION=off`.
7. **Search ranking:** a content query returns content before never-used system rows (the `cmp_v0` change); compare with `keyword_hits.jsonl` growth as before.
8. Memory/CPU of the host for the first hour (the work-order tree adds task records per walk; the bridge `select!` loop only runs when publishing is on).

### 7a. Turning each feature on later (one at a time, each with its own walk)
* Tool creation: remove `OZONE_TOOL_CREATION=off` (or set `on`), restart, run a request that reports a missing capability. Expect: `tool creation: tool created, registered (stdio, never auto-started) and published as a finding`; `zsei_data/created_tools/<slug>/{tool.json,implementation.mjs,origin.json}`; a row in `mcp_tool_registry.json` with `transport: stdio`, `auto_start: false`; `capability_gaps.json` row gets `tool_created`, `tool_registered`, `satisfied`, `created_tool`; the gap's work-order task ticks `tool` and `register`. Nothing publishes while `marketplace_publish` is false.
* Refinement: remove `OZONE_REFINEMENT_ACTION=off` while keeping `OZONE_REFINEMENT_RETIRE=off` first (split only), inspect `methodology_refinement_actions.jsonl`, then remove `OZONE_REFINEMENT_RETIRE=off` (retire needs the rebuilt `methodology_create`; an old binary makes the record say `refused` honestly).
* Marketplace: `marketplace_publish` and `marketplace_accept_offers` separately (MASTER_TEST_GUIDE §27k Q1–Q4; §27k Q3 was corrected in this pass: there is no `offer:` item).

## 8. Test suites after the swap (EU; device key at `~/.ozone-cli/device-key.pem`, `OZONE_REQUIRE_SESSION=1` is handled by `lib.mjs`)
```bash
cd /opt/ozone-build
export OZONE_DATA=/opt/ozone-studio/zsei_data          # suites default to target/release/zsei_data, which is NOT the live data dir on the EU
node tools/e2e/keyword-program.mjs                      # K1–K9: previous EU result 9 pass / 0 fail / 1 skip. A change here means the tag namespacing or cmp_v0 moved something — read the row, do not re-fit the threshold.
node tools/e2e/span-walk.mjs                            # scope tests; SPAN_DIR must be a dir the host can read (the service has PrivateTmp)
WALK_MAX_MS=7200000 node tools/e2e/walk-completion.mjs  # the one real walk (needs quota); writes walk_metrics.jsonl rows
node tools/e2e/marketplace.mjs                          # with the marketplace OFF: record what it reports honestly; do not turn the switches on to "pass" it
tools/e2e/rerun-all.sh                                  # full ordered list: modalities mcp-contract jurisdiction marketplace fanout meta-ripple methodology-sufficiency provider-step keyword-program span-walk walk-completion; summary in docs/evidence/test-runs/<date>-rerun-summary.md
```
Known interaction to expect: `jurisdiction.mjs` may now see `Log` rules as warnings (intended; unit test updated). Suites that read bookkeeping tags by plain word will see `kind:` markers on NEW rows — `tools/e2e/*.mjs` contain no such assertion (grep), `tools/ozone-shared-context/server.js` reads both forms.
Record each suite in `docs/evidence/test-runs/<date>-<suite>.md` and update the ✔/☐ marks in `MASTER_TEST_GUIDE` in the same commit (guide §16).

## 9. Expected duration (from the previous EU runs — this diff is not measured)
check ≈ 3–5 min · host build ≈ 5.5–6.5 min · lib tests ≈ test-profile compile + run, several minutes (never timed in the records; the previous full run was one compile + 258 tests) · 3 pipeline crates ≈ 2–4 min each (test + build) · swap ≈ 2 s of downtime · K-suite ≈ 2–6 min · one real walk 40 min – 2 h on the free tier · `rerun-all.sh` up to `SUITE_TIMEOUT` (3000 s) per suite.

## 10. LEFT FOR REVIEW (dead-looking items I did NOT remove — operator ruling: remove only with an identified superseder, never anything docs-registered)
Removed in this pass, each with its superseder: see STATUS §V. The following look dead but were left; for each: where, why it looks dead, what supersedes it, why it stayed.
| Item | file:line | Looks dead because | Superseder (identified) | Why NOT removed |
|---|---|---|---|---|
| `load_methodology_rules_text` | `src/orchestrator/amt.rs:2557` | no code caller (only comments); identical body to `_full` | `load_methodology_rules_full` (amt.rs:2566) and `PromptOrchestrator::rules_text_for` (stages.rs) — audit item S1 of `DOCTRINE_VIOLATIONS_AUDIT_2026-11-04.md` said "delete the limited variant" | docs-registered: `CONTEXT_REGISTRY.md:23,70`, `BUILDER_REGISTRY.md:363`, S1 audit rows. Will warn `dead_code` (pub(crate), unused) — harmless. Remove together with a registry edit if wanted. |
| `NetworkManager::on_item_announced` | `src/network/mod.rs:1028` | no caller (the bridge calls `on_items_announced`); only the hook-name string `"on_item_announced"` remains in use | `on_items_announced` (commit 113bf0b, STATUS §Q coalesced publish) | `pub` API; the single-item wrapper is described in STATUS §B2 and CHECKLIST L75 |
| `NetworkManager::publish_to_marketplace` | `src/network/mod.rs:1095` | no caller (only `on_items_announced` → `publish_batch_to_marketplace`) | `publish_batch_to_marketplace` (§Q) | `pub`; named in STATUS §B2 table (docs-registered) |
| `NetworkManager::process_batch_sync` | `src/network/mod.rs:1194` | the periodic driver that called it was removed from `lib.rs` (commit 860d27a/113bf0b: "nothing feeds the Normal-priority per-peer queue any more") | marketplace publish (one push per batch) | `pub`; part of the P2P layer's own API/tests surface — an operator decision |
| `on_methodology_created`, `on_blueprint_created`, `on_container_created` | `src/network/mod.rs:1000,1017,1125` | no caller since `spawn_network_hook_bridge` was rewritten to manifests | `on_items_announced` (`AnnounceItem`) | `pub`; `on_blueprint_created` is cited in STATUS §B2 ("no-op — blueprints are private"); their hook-name strings are registered at `network/mod.rs:896,909,933` |
| `.owo-*` classes other than the ones in use | `ui/src/App.css` ~7595-7615 | — | — | checked: every remaining `owo-*` class is used by `WorkOrderTree.tsx`; nothing else to remove |
Items the directive listed that were already gone before this pass: `read_json_array_names` (grpc; replaced by `peer_import::LocalOfferIndex::load`, commit 113bf0b — no reference left), `offers_for_gap` and the `offer:` work-order item (removed in e7bbdc5, UI in 881d2ea — no reference left in `src/`, `ui/src`).
