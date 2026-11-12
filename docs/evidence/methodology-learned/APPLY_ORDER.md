# Apply order (nothing here has been run)

Apply is a data mutation on the operator's store: only when the operator says, local first, then the EU (`feedback_local_first_then_sync_eu`). Follow `live-data-changes-copy-check-apply-verify-restore` (draft 39) for the procedure.

## Preconditions

1. The build gate is unlocked and the build, deploy and first real walk are done (BUILD_AND_TEST_PLAN section 12 apply phase).
2. The deployed `methodology_create` binary carries the new crate (`remove_decision_rules`, from commit 88d700a or later). `apply.mjs` fails closed on an old crate.
3. EU service healthy; settlement still paused by the operator drop-in unless the operator lifted it.

## Steps

1. **Set 1: drafts 01-30** (`docs/evidence/methodology-from-prompts/drafts`). Dry run, read it, then `--apply` only on the operator's go. Drafts 14 and 26 are `confirmed`; the others are `ready`.
2. **Set 2, creates: drafts 31-49** (`node tools/methodology-from-prompts/apply.mjs --drafts docs/evidence/methodology-learned/drafts --only <slugs>`). They are independent of each other and of set 1, so they can run in the same session. Dry run first: any same-name refusal means merge, not create.
3. **Set 2, updates: drafts 50-57, in a SECOND run after steps 1 and 2.** `apply.mjs` resolves update targets from a name snapshot taken at the start of a run, so an update whose target is created in the same run is refused; run it again after the creates exist. Order inside this pass: 50, 51, 52, 54, 55, 56, 57 (each on its own set-1 target), then 53 after draft 28 (same target, second update).
4. After each write the script reads the container back and records the slug in `applied.json` with before-copies and a rollback file. Check health between methodologies; stop at the first failure (the script does).
5. **Hand-validate the new terms.** New containers register their keywords as PENDING and strict search ignores pending terms, so none of these methodologies can be found by strict search until validated. Run the hand-validation (draft 40) on the new and updated containers right after each batch; settlement stays paused.
6. **Verify independently** (draft 39): reload each container, grep the store for the slugs, check the journal; do not rely on the script's output alone.
7. **Re-register git-mcp** after its rewrite deploys, so the registry knows its current tool names; drafts 38 and 44/49 only name `file_history`, `file_diff` and `code_callgraph`, which the rewritten git-mcp keeps (aliases) and code-mcp provides.

## Decisions to take before applying

- Overlaps flagged in INDEX.md: 31 vs 12/11, 35 vs method_22/23, 39 vs 15, 40 vs 07, 41 vs method_33. Each can be applied as is (separate methodology), or folded into the existing one as an update; the drafts are written so either works.
- Draft 51 records the operator's instruction to bring the bridges console under the host. It is a methodology about the pattern; the code change itself is separate and not done.
- Withheld items (quota gating; MCP as a model): see INDEX.md.

## Rollback

Use the `before-<stamp>` folder and `rollback.json` that `apply.mjs` writes (created: delete through pipeline 12; updated: point `storage.object_store_path` back at the saved body). The restore path has not been exercised.
