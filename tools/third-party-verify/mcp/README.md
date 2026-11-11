# MCP-driven verification (three agents, real servers)

Drafted 2026-11-11, **syntax-checked only, NOT RUN.** A stdio JSON-RPC client (`harness.mjs`) spawns the real
`ozone-shared-context` and `ozone-mcp` servers for three agents (`claude-code`, `zcode`, `third-agent`) that share one
throwaway context dir, against a real running host. No mocks; every row prints the value actually read.

```
node tools/third-party-verify/mcp/m1-coordination.mjs        # read-mostly: surface, presence/claims/notes handoff, identity, refusals, chunking
OZONE_TPV_WALK=1 node tools/third-party-verify/mcp/m2-walk.mjs "<prompt>"    # opt-in (model calls): A starts, B/C observe, completion gate, live call graph
OZONE_PROJECT_ID=<id> [OZONE_ZSEI_PARENT_ID=<id>] [OZONE_TPV_WALK=1] node tools/third-party-verify/mcp/m3-link-modalities.mjs
                                                              # mutating (links fixtures): link-first non-semantic graph per modality, then walk reuse
```
Env: `OZONE_HOST`, `OZONE_TPV_MAX_MS` (default 600000 wait for async work), `OZONE_WORKSPACE_ID`, `OZONE_PROJECT_ID`.
Evidence: `docs/evidence/third-party-verify/<date>-tpv-mcp-*.md`. Exit 1 = at least one FAIL.

Order vs `docs/BUILD_AND_TEST_PLAN_2026-11-10.md`: after the build + deploy and `tools/third-party-verify/01`, run `m1`; `m3`
(without the walk) checks graph-on-link; `m2` and `m3` with the walk are the first real walk. Fixtures exist for text, code,
image, audio, 3d (obj, stl); other modalities are SKIPPED, not faked. Expect shape mismatches on the first live run — fix the
script from the real payload, never loosen a check.
