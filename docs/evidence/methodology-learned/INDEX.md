# Methodologies from learned lessons - index (2026-11-12)

Docs/data only. 19 new methodologies (31-49) and 8 add-only updates (50-57), drafted from the evidence reports, decision log, operator memory and status documents. **Nothing was applied: apply.mjs was not run, the EU and the local store are untouched, nothing was built or tested.**

Same format as `docs/evidence/methodology-from-prompts/drafts/` (the 30 earlier drafts, 01-30, which these do NOT duplicate): top-level `slug`, `kind`, `status`, `target_name`, `source_quotes`, `overlap_check`, `memory_links`, plus a new `evidence` list (file:line or section for every draft), and the real `methodology_create` input in `payload`. Applied by `tools/methodology-from-prompts/apply.mjs --drafts docs/evidence/methodology-learned/drafts` (dry run by default).

Lint run on every draft (all clean): no 4-5 digit literals, no confidence wording (outside the one update about retiring the confidence field), keywords of at most three words, rule names unique within a draft and not equal to a rule name in drafts 01-30, `requires` only on tools registered on the EU port table (`file_history`, `file_diff` on git-mcp, `code_callgraph` on code-mcp) and always optional, every draft carries evidence.

## Drafts

| # | draft | kind | name / target | rules | sources (evidence) | relation to existing | overlap / proposal |
|---|---|---|---|---|---|---|---|
| 31 | no-skip-fix-or-pause-and-retry | create | There Is No Skip: Fix It or Pause and Retry | 6 | docs/evidence/no-skip-sweep/REPORT.md:42; docs/evidence/no-skip-sweep/REPORT.md:58; docs/evidence/final-decisions/DECISIONS.md:31 ... | 12 (gates await), 11 (pause at failed step), built-in method_43, method_19 | OVERLAP: partial. Kept separate (trigger = any path that records a failure and continues). Review option: fold rules 1-3 under 12. |
| 32 | acceptance-is-readability | create | Only What Can Be Read Is Linked | 6 | docs/evidence/no-skip-sweep/REPORT.md:16; docs/evidence/final-decisions/DECISIONS.md:32; docs/evidence/final-decisions/DECISIONS.md:44 | none | new |
| 33 | invariants-assigned-by-construction | create | Establish Invariants by Construction, Not by Failing | 5 | docs/evidence/no-skip-sweep/REPORT.md:7; docs/evidence/final-decisions/DECISIONS.md:30; docs/evidence/final-decisions/DECISIONS.md:8 | 22 (blueprint steps), 31 | new |
| 34 | never-mint-an-identifier | create | Identifiers Come From the Store, Never From a Clock | 4 | docs/evidence/graph-link-walk-alignment/REPORT.md:44; docs/evidence/walk-finalization/REPORT.md:13; docs/evidence/no-skip-sweep/REPORT.md:42 | 03 (empty is a failed call), 31 | new |
| 35 | durable-ordered-writes | create | Writes That Matter Are Durable, Ordered and Never Disable Their Sink | 6 | docs/evidence/no-skip-sweep/REPORT_D6_COMPLETE.md:6; docs/evidence/no-skip-sweep/REPORT_D6_COMPLETE.md:49; docs/evidence/no-skip-sweep/REPORT_D6_COMPLETE.md:57 ... | built-in method_22, method_23, method_39; 31, 38 | OVERLAP: partial with method_22/23 (cache and check durability). Write path itself is new. |
| 36 | resource-floors-pause-never-skip | create | Resource Floors Pause the Work, They Never Skip or Refuse It | 4 | docs/evidence/walk-finalization/REPORT.md:12; docs/evidence/final-decisions/DECISIONS.md:11; docs/evidence/guardrail-audit/REPORT.md:24 | 02 rule 5 (name a host guard), 31 | new (behaviour half of 02's naming rule) |
| 37 | link-first-walk-adds-intelligence | create | Link Captures the Structure First, the Walk Adds Intelligence on One Shared State | 7 | docs/evidence/graph-link-walk-alignment/REPORT.md:17; docs/evidence/graph-link-walk-alignment/REPORT.md:32; docs/evidence/dead-code-removal/REPORT.md:26 ... | 26 (modality call graph rule), built-in method_41, method_24 | new |
| 38 | history-chunked-verified-reversible | create | Version History Is Chunked, Verified and Reversible | 7 | docs/HISTORY_STORE_GUIDE.md:10; docs/HISTORY_STORE_GUIDE.md:26; docs/HISTORY_STORE_GUIDE.md:34 ... | built-in method_22 (in-memory index vs durable store), 35 | OVERLAP: the restart-safety lesson is method_22's; the store design is new. |
| 39 | live-data-changes-copy-check-apply-verify-restore | create | Changing Live Data: Look, Copy, Check, Apply One, Verify, Keep a Way Back | 8 | docs/evidence/methodology-duplicate-delete/REPORT.md:5; docs/evidence/methodology-duplicate-delete/REPORT_PHASE2.md:5; docs/evidence/methodology-duplicate-delete/REPORT_PHASE2.md:23 ... | 15 rule 5 (keep data changes reversible), built-in method_33 | OVERLAP: 15 rule 5 is the one-line form. Review option: point 15 rule 5 here. |
| 40 | hand-validation-when-settlement-paused | create | Validate Pending Keywords by Reading When Settlement Is Paused | 6 | docs/evidence/keyword-validation/methodology/SUMMARY.md:5; docs/evidence/keyword-validation/methodology/SUMMARY.md:16; docs/evidence/keyword-validation/round2/SUMMARY.md:6 ... | 07 (keyword capture validated and hit driven) | OVERLAP: procedure beside 07's gate. Review option: make these rules of 07 (update) instead of a separate methodology. |
| 41 | report-what-was-done-at-the-level-done | create | Report Work at the Level It Was Actually Done | 5 | AGENTS.md; docs/evidence/final-decisions/DECISIONS.md:48; docs/STATUS_2026-11-12.md ... | built-in method_33, method_20; 13, 16 | OVERLAP: real. Review option: fold into method_33 as an update. |
| 42 | coordinate-agents-through-shared-context | create | Coordinate Agents Through the Shared Context | 6 | AGENTS.md; tools/ozone-shared-context/README.md; docs/evidence/mcp-participants/REPORT.md:22 | built-in method_20, method_50 | new |
| 43 | every-entry-point-names-its-credential | create | Every Entry Point Names Its Credential and Protects Its Mutations | 7 | docs/evidence/mcp-participants/REPORT.md:28; docs/evidence/dead-code-removal/REPORT.md:23; docs/STATUS_2026-11-12.md:50 ... | built-in method_5, method_47, method_48 | new (credential matrix and protected-list rule) |
| 44 | review-every-call-site-against-the-gate | create | Review Every Call Site Against the Gate, Not Only the Obvious Route | 8 | docs/evidence/jurisdiction-deepening/CALL_SITE_INVENTORY.md:5; docs/evidence/jurisdiction-deepening/CALL_SITE_INVENTORY.md:30; docs/evidence/jurisdiction-deepening/REPORT_PHASE3.md:6 ... | 12, 45, 43 | new (coverage review; 12 is gate behaviour) |
| 45 | jurisdiction-per-instance-policy | create | Jurisdiction Is a Per-Instance Policy With a Durable Record | 7 | docs/evidence/jurisdiction-deepening/REPORT.md:13; docs/evidence/jurisdiction-deepening/REPORT.md:38; docs/evidence/jurisdiction-deepening/REPORT.md:40 ... | built-in method_21, method_34; 12 | new (per-instance policy, agreements, decision log) |
| 46 | mcp-agents-participate-additively | create | MCP Agents Take Part in a Walk Without Becoming Part of Its Control Flow | 6 | docs/evidence/mcp-participants/REPORT.md:7; docs/evidence/mcp-participants/REPORT.md:22; docs/evidence/mcp-participants/REPORT.md:28 ... | 09, 13, built-in method_50 | new; MCP-as-a-model is NOT covered (not built) |
| 47 | host-called-pipelines-accept-envelope-and-flat | create | Host-Called Pipelines Accept the Envelope and Flat Input | 3 | memory: project_pipeline_envelope_unwrap; src/orchestrator/adapters.rs | built-in method_40 | new (call shape; 40 is storage shape) |
| 48 | measure-and-batch-our-own-judgment-volume | create | Measure and Batch the Judgment Calls We Generate Ourselves | 3 | memory: project_quota_economics_and_meta_starvation; docs/evidence/mcp-participants/REPORT.md:28 | 23, 28, built-in method_38 | new (3 rules); the quota-gating rule is deliberately withheld, see below |
| 49 | generated-facts-extracted-with-drift-check | create | Facts About the Code Are Generated, Checked for Drift and Honest About Limits | 5 | docs/evidence/call-graphs/REPORT.md:16; docs/evidence/call-graphs/REPORT.md:18; docs/evidence/call-graphs/REPORT.md:26 ... | 26, built-in method_38 | new |
| 50 | refine-work-order-expansion-and-gates | update | Work Order Completion Is Final | 4 | docs/evidence/walk-expansion-completion/REPORT.md:9; docs/evidence/walk-expansion-completion/REPORT.md:23; docs/evidence/walk-finalization/REPORT.md:8 ... | 11 | UPDATE of draft 11 (apply after set 1) |
| 51 | refine-ozone-first-fold-in-redundant-components | update | Ask What Ozone Already Owns First | 4 | docs/STATUS_2026-11-12.md:37; operator message 2026-11-12; docs/evidence/jurisdiction-deepening/REPORT_PHASE3.md:6 | 19 | UPDATE of draft 19 (apply after set 1) |
| 52 | refine-verify-through-ozone-no-mocks | update | Verify Ozone Through Ozone as a Third Party | 3 | memory: feedback_no_mocks_test_real; memory: feedback_ozone_doctrines_2026_11; docs/BUILD_AND_TEST_PLAN_2026-11-10.md | 13, built-in method_49 | UPDATE of draft 13 (apply after set 1); close to method_49 |
| 53 | refine-stop-tuning-a-working-parameter | update | Tune From Measured Metrics, Never Preemptive Caps | 1 | memory: feedback_dont_oververify_working_params | 28 (same target), built-in method_38 | UPDATE of the built-in/EU methodology 'Tune From Measured Metrics, Never Preemptive Caps'; second update on that target after 28 |
| 54 | refine-p2p-paired-devices-vs-marketplace | update | Peer Marketplace Notifies and Gates | 4 | memory: feedback_sync_no_action_is_paired_devices_not_p2p; docs/evidence/jurisdiction-deepening/REPORT_PHASE3.md:6; docs/evidence/jurisdiction-deepening/REPORT_PHASE2.md:7 ... | 09 | UPDATE of draft 09 (apply after set 1) |
| 55 | refine-remove-superseded-lessons-from-the-pass | update | Remove Only What Is Shown To Be Superseded | 5 | docs/evidence/dead-code-removal/REPORT.md:6; docs/evidence/dead-code-removal/REPORT.md:18; docs/evidence/final-decisions/DECISIONS.md:17 ... | 17, built-in method_24 | UPDATE of draft 17 (apply after set 1) - the dead-code removal lessons |
| 56 | refine-protocol-constants-and-spacing | update | Chunk to the Reading Model's Window | 2 | docs/evidence/walk-finalization/REPORT.md:10; docs/evidence/no-skip-sweep/REPORT_D6_COMPLETE.md:6; docs/evidence/final-decisions/DECISIONS.md:9 | 02 | UPDATE of draft 02 (apply after set 1) |
| 57 | refine-retire-a-persisted-field | update | Binary Judgments and Measured Numbers | 2 | docs/evidence/jurisdiction-deepening/REPORT_PHASE2.md:21; memory: feedback_no_self_reported_confidence; memory: feedback_confirm_ambiguous_revert_direction | 01, 24 | UPDATE of draft 01 (apply after set 1) |

## Where the lessons you listed ended up

| lesson | where |
|---|---|
dead code removed only when a superseder is cited | draft 17 already (ready, not yet applied) + NEW update 55 with what the first real removal pass taught
no-skip: failures pause and retry | NEW 31 (+32 acceptance, 33 invariants, 34 minted ids, 36 resource floors)
local first then sync EU with diff | draft 15 already; NEW 39 adds the live-data change procedure
parse-only edits reviewed like a compiler | draft 16 already
review call sites for gate bypasses | NEW 44
no self-reported confidence | draft 01 already + NEW update 57 (how to retire the persisted field)
chunk, never refuse by size; no caps | drafts 02/28 already + NEW update 56 (protocol constants, backoff is spacing)
durable writes | NEW 35 (+38 history)
credentials / internal-token matrix | NEW 43
history / restore reversibility | NEW 38
evidence-first status, no fabricated metrics, fork-report handling | NEW 41
walk/link ledger alignment | NEW 37 (+ update 50 for expansion and the completion gate)
jurisdiction per-instance policy | NEW 45
hand validation of PENDING keywords | NEW 40
MCP agents in walks | NEW 46
fold redundant components (bridges console) into the host; keep agent integrations current | NEW update 51

## Deliberately NOT drafted (needs the operator)

1. **Background actors wait on a known quota exhaustion.** The 2026-11-08 finding (memory `project_quota_economics_and_meta_starvation`) says gate every background actor on the quota; draft 23 says let quotas run out and the operator ruled no quota sharing or gating (`mcp-participants` H9 not wired). The three uncontested rules of that finding are in draft 48; this one is left out until the operator resolves the conflict.
2. **MCP as a model** (an MCP tool as a selectable model in the model chain). Not built; the operator clarification of 2026-11-12 is recorded in `docs/STATUS_2026-11-12.md` section F and noted in draft 46's overlap field. A methodology about it should wait for the design.
3. **Legal conclusions.** Draft 45 records how the policy surface behaves and labels it engineering input; it states no legal claim. Dates and case numbers flagged in `jurisdiction-deepening/REPORT.md` remain to be verified.
4. **Sensor-modality raw input** is open (deferred); it appears only as a rule inside draft 32 ('reader first, tracked as open').
5. **Per-pipeline facts** (which pipeline calls which provider) are in `CALL_SITE_INVENTORY.md`, not in a methodology, because they change; draft 44 holds the method for producing them.

## EU overlap check still to run

I could not read the EU store from here (docs only). `apply.mjs` refuses a Create whose name already exists and an Update whose target does not resolve to exactly one methodology, so the dry run on the EU is the real overlap check; the names in this index were compared with drafts 01-30, the 50 built-in methodologies and the rules review (98 EU methodologies, `methodology-rules-review/REPORT.md`) by reading only.
