# Final decisions report

- Keyword toggles: IGNORE_EXPANSION (keyword_program.rs:225-241) = A/B ablation ignoring terms whose keyword_meta.source is "expansion"; STOPLIST (request_terms.rs:10-24) = ~90 fixed function words dropped. Both stay OFF; they make the system do less when on.
- Strict search + paused settlement interact: new containers' terms register PENDING (zsei/query.rs:495-499), strict search ignores PENDING (keyword_program.rs:219-224). Validate by hand after each container-creating batch, or lift the pause.
- Peer methodology recheck: none (decision; guardrail-audit R2 recorded as decided).
- Duplicate scripts: tools/methodology-duplicate-repoint/ (dry run default; apply phase only).
See DECISIONS.md and docs/BUILD_AND_TEST_PLAN_2026-11-10.md.
