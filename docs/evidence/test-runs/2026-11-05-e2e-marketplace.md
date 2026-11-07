# e2e-marketplace — 2026-11-05

host: http://127.0.0.1:50051 · duration 0s · PASS 16 · FAIL 0 · SKIP 0

| result | id | test | evidence |
|---|---|---|---|
| PASS | MK0 | overview succeeds | ["binding_proposals","blueprints","generated_at","methodologies","peers","policy","provider_health","success","tools","wanted"] |
| PASS | MK1 | tool count equals the live registry | 184 vs 184 |
| PASS | MK2 | every registered tool appears in exactly one PROVIDER (MCP server id) | 24 providers |
| PASS | MK2b | threed-mcp is ONE provider backing clock_generate + cad_clock_generate |  |
| PASS | MK3 | providers report REAL running state (clock_generate provider matches its port) |  |
| PASS | MK4 | every methodology file on disk is listed | 59 vs 59 files |
| PASS | MK5 | rules come with their text + a binding status | 96 rules |
| PASS | MK6 | declared per-rule bindings counted from the methodology files (not inferred) | 0 of 96 rules declare tools |
| PASS | MK7 | keyword-inferred PROVIDERS are real registered providers and only on rules WITHOUT a declaration | 64 rules with inferred providers |
| PASS | MK8 | every blueprint file on disk is listed | 31 vs 31 |
| PASS | MK9 | blueprint rows are real files; declares_tool_calls is only true for blueprints that actually contain provider steps |  |
| PASS | MK9b | wanted / provider_health / binding_proposals are present and well-formed | 0 wanted |
| PASS | MK10 | paired peers match /network/paired-peers | 1 paired |
| PASS | MK11 | peer catalog exchange is reported honestly as not implemented (no invented listings) |  |
| PASS | MK12 | policy flags present (review + executables need human approval) |  |
| PASS | MK13 | route reachable on this host config (status recorded) | HTTP 200 (401 expected only when OZONE_REQUIRE_SESSION=1) |
