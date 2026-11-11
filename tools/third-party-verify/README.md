# Third-party verification (outside client)

Verifies Ozone-Studio the way a user/agent of it would: only through its public HTTP surface, with the same device-key session as `tools/ozone-cli` / `tools/e2e/lib.mjs`. It never reads the code tree or the data dir and never edits the store. No mocks; every row prints the value actually read; absent things are SKIP with the reason; shape mismatches are FAILs to investigate, not to loosen.

```
node tools/third-party-verify/01-surface.mjs      # health, session, routes
node tools/third-party-verify/02-work-order.mjs   # /task/list + /order/work/:id
node tools/third-party-verify/03-gaps-and-refinement.mjs
node tools/third-party-verify/04-call-graphs.mjs  # OZONE_PROJECT_ID=<id> to restrict
node tools/third-party-verify/05-marketplace-views.mjs   # never calls offers/decide
OZONE_TPV_WALK=1 node tools/third-party-verify/06-walk.mjs "<prompt>"   # opt-in, spends model calls
node tools/third-party-verify/07-tools-and-coordination.mjs
tools/third-party-verify/run-all.sh               # 01-05 + 07
```
Env: `OZONE_HOST` (default http://127.0.0.1:50051), `OZONE_USER_ID` (04), `OZONE_TPV_MAX_MS` (06).
Evidence: `docs/evidence/third-party-verify/<date>-<suite>.md`. Exit code 1 = at least one FAIL.

Order vs `docs/BUILD_AND_TEST_PLAN_2026-11-10.md`: after §5 (install+start) run 01, 03, 05, 07 as the baseline; 06 is the §7 first real walk; then 02, 03, 04 to read its effects; §8 `tools/e2e` suites follow. Details: `docs/evidence/tests-in-code/REPORT.md` §5. Status: written, syntax-checked only, NOT RUN.
