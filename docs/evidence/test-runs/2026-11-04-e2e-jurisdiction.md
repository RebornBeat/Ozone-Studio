# e2e-jurisdiction — 2026-11-04

host: http://127.0.0.1:50051 · duration 2s · PASS 9 · FAIL 0 · SKIP 0

| result | id | test | evidence |
|---|---|---|---|
| PASS | J1 | Block rule ('exploit') refuses the MCP call before invoke | {"error":"MCP call blocked by jurisdiction gate: a Block-action jurisdiction rule matched","jurisdiction_gate":{"blocked":true,"confirmations":[],"matched":[[{"action":"Block","aliases":[],"condition":"exploit","official |
| PASS | J1b | Block rule ('abuse') refuses | {"error":"MCP call blocked by jurisdiction gate: a Block-action jurisdiction rule matched","jurisdiction_gate":{"blocked":true,"confirmations":[],"matched":[[{"action":"Block","aliases":[],"condition":"abuse","official_s |
| PASS | J2 | benign input passes, tool answers | {"error":null,"jurisdiction_gate":{"blocked":false,"confirmations":[],"matched":[],"pending_reviews":[],"rules_loaded":114,"rules_source":"","warnings":[]},"output":{"output":{"current_datetime_utc":"2026-11-04T00:20:28. |
| PASS | J3 | Warn rule ('discriminate') does NOT block the call | {"error":null,"jurisdiction_gate":{"blocked":false,"confirmations":[],"matched":[[{"action":"Warn","aliases":[],"condition":"discriminate","official_source":null,"retrieved_snippet":null,"scope":"Global","source":"UDHR A |
| PASS | J3b | Warn is surfaced in the response (jurisdiction warning present) | {"error":null,"jurisdiction_gate":{"blocked":false,"confirmations":[],"matched":[[{"action":"Warn","aliases":[],"condition":"discriminate","official_source":null,"retrieved_snippet":null,"scope":"Global","source":"UDHR A |
| PASS | J4 | RequireConfirmation ('self-harm') is reviewed (decision recorded, not skipped) | {"error":null,"jurisdiction_gate":{"blocked":false,"confirmations":[[{"action":"RequireConfirmation","aliases":[],"condition":"self-harm","official_source":null,"retrieved_snippet":null,"scope":"Global","source":"UDHR Ar |
| PASS | J5 | /pipeline/execute refuses a Block phrase (blocked by the jurisdiction gate) | {"success":false,"task_id":0,"output":null,"error":"blocked by the jurisdiction gate: step blocked by jurisdiction rule (matched \"exploit\"): exploit [CRC Article 34 (protection from sexual exploitation and abuse)]"} |
| PASS | J6 | jurisdiction config readable (enabled, region) | {"success":true,"config":{"enabled":true,"instance_region":"DO"},"error":null} |
| PASS | J7 | awaiting jurisdiction items listed (informational) | 0 item(s) |
