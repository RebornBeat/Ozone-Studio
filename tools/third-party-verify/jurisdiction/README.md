# Jurisdiction verification (third party, not run)

Drives the real public HTTP and MCP surface. Nothing here lives in production code; nothing is mocked; every row prints the value it read.
These scripts never accept a requirement and never approve a transfer on their own — those are the operator's decisions (j3 can approve ONE content hash only with `OZONE_TPV_J_APPLY=1`).

| Script | Mode | Checks |
|---|---|---|
| `j1-policy-surface.mjs` | read-only | policy shape, no confidence fields, requirements status, decision-log shape (no content, append-only order), export shape |
| `j2-gate-behaviour.mjs` | `OZONE_TPV_WALK=1` | Block / Warn / Allow prompts through `/orchestrate`; decision rows (hashes only); blocked walk did not run steps |
| `j3-outbound-gate.mjs` | `OZONE_TPV_J_OUTBOUND=1` | personal-data text bound for a third-country destination is HELD (AWAITING_OPERATOR) not sent; rows carry kinds only |
| `j4-mcp-surface.mjs` | `OZONE_TPV_WALK=1 OZONE_PROJECT_ID=` | MCP orchestrate / link behave the same as HTTP; participants see the walk; rows for every action |
| `j5-phase2.mjs` | read-mostly | region visibility (G12), publish-kinds policy refusal (G5), review entry points in the decision log (G2) |

Order: after the build gate and first deploy, run j1; then (with the operator present) accept the requirements in the UI; then j3, j2, j4. On the first boot of this build, outbound calls to third-country providers are HELD until the requirements are accepted — that is the policy default working, not a fault.
