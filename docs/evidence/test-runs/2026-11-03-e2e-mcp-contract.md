# e2e-mcp-contract — 2026-11-03

host: http://127.0.0.1:50051 · duration 8s · PASS 43 · FAIL 0 · SKIP 2

| result | id | test | evidence |
|---|---|---|---|
| PASS | terminal-mcp.registry | registry endpoint is :3215 | http://127.0.0.1:3215/call |
| PASS | terminal-mcp.positive | terminal_status succeeds with a real answer | {"output":{"allowlist_legacy_global":[],"locked":false,"platform":"linux/x64","roles_configured":["_ |
| PASS | terminal-mcp.ripple | the call carries the captured block (usage ledger + jurisdiction gate + graph ripple) | ["usage ledger row recorded","jurisdiction gate applied","graph ripple emitted for the call"] |
| PASS | terminal-mcp.refusal | terminal_exec bad input → clean success:false + reason (no 5xx, no crash) | command 'rm' is not in the allowlist for role of 'e2e' (ls) |
| PASS | terminal-mcp.s13 | S13 capture holds a row for terminal_status | rows=2 |
| PASS | web-mcp.registry | registry endpoint is :3225 | http://127.0.0.1:3225/call |
| PASS | web-mcp.positive | current_datetime succeeds with a real answer | {"output":{"current_datetime_utc":"2026-11-03T12:59:25.807772565+00:00","error":null,"provider_used" |
| PASS | web-mcp.ripple | the call carries the captured block (usage ledger + jurisdiction gate + graph ripple) | ["usage ledger row recorded","jurisdiction gate applied","graph ripple emitted for the call"] |
| PASS | web-mcp.refusal | web_search bad input → clean success:false + reason (no 5xx, no crash) | query required |
| PASS | web-mcp.s13 | S13 capture holds a row for current_datetime | rows=8 |
| PASS | code-mcp.registry | registry endpoint is :3230 | http://127.0.0.1:3230/call |
| PASS | code-mcp.positive | code_callgraph succeeds with a real answer | {"output":{"call_sequence":[{"called_from":null,"depth":0,"function":"a","line":1,"order":0},{"calle |
| PASS | code-mcp.ripple | the call carries the captured block (usage ledger + jurisdiction gate + graph ripple) | ["usage ledger row recorded","jurisdiction gate applied","graph ripple emitted for the call"] |
| PASS | code-mcp.refusal | code_callgraph bad input → clean success:false + reason (no 5xx, no crash) | code required |
| PASS | code-mcp.s13 | S13 capture holds a row for code_callgraph | rows=11 |
| PASS | git-mcp.registry | registry endpoint is :3235 | http://127.0.0.1:3235/call |
| PASS | git-mcp.positive | file_status succeeds with a real answer | {"output":{"watched":[{"changes":183,"container_id":"40380","last_change_ts":1793678333965,"path":"/ |
| PASS | git-mcp.ripple | the call carries the captured block (usage ledger + jurisdiction gate + graph ripple) | ["usage ledger row recorded","jurisdiction gate applied","graph ripple emitted for the call"] |
| SKIP | git-mcp.refusal | no refusal path exercised | tool accepted the probe |
| PASS | git-mcp.s13 | S13 capture holds a row for file_status | rows=5 |
| PASS | security-mcp.registry | registry endpoint is :3240 | http://127.0.0.1:3240/call |
| PASS | security-mcp.positive | security_status succeeds with a real answer | {"output":{"privileged":false,"read_only":true,"tool":"security-mcp","tools":["net_connections","lis |
| PASS | security-mcp.ripple | the call carries the captured block (usage ledger + jurisdiction gate + graph ripple) | ["usage ledger row recorded","jurisdiction gate applied","graph ripple emitted for the call"] |
| SKIP | security-mcp.refusal | no refusal path exercised | tool accepted the probe |
| PASS | security-mcp.s13 | S13 capture holds a row for security_status | rows=2 |
| PASS | dna-mcp.registry | registry endpoint is :3271 | http://127.0.0.1:3271/call |
| PASS | dna-mcp.positive | dna_sequence_info succeeds with a real answer | {"output":{"accession":"NC_000908.2","cds_count":524,"gc_content_percent":31.689,"gene_count":566,"l |
| PASS | dna-mcp.ripple | the call carries the captured block (usage ledger + jurisdiction gate + graph ripple) | ["usage ledger row recorded","jurisdiction gate applied","graph ripple emitted for the call"] |
| PASS | dna-mcp.refusal | dna_translate bad input → clean success:false + reason (no 5xx, no crash) | invalid DNA sequence — unexpected character(s): ['!', 'X', 'Y', 'Z'] |
| PASS | dna-mcp.s13 | S13 capture holds a row for dna_sequence_info | rows=3 |
| PASS | chemistry-mcp.registry | registry endpoint is :3272 | http://127.0.0.1:3272/call |
| PASS | chemistry-mcp.positive | chem_analyze succeeds with a real answer | {"output":{"atoms":[{"atom_id":0,"atomic_number":6,"element":"C","explicit_hydrogens":0,"formal_char |
| PASS | chemistry-mcp.ripple | the call carries the captured block (usage ledger + jurisdiction gate + graph ripple) | ["usage ledger row recorded","jurisdiction gate applied","graph ripple emitted for the call","graph persisted: root 4408 |
| PASS | chemistry-mcp.refusal | chem_analyze bad input → clean success:false + reason (no 5xx, no crash) | RDKit could not parse this SMILES string — not a valid molecule |
| PASS | chemistry-mcp.s13 | S13 capture holds a row for chem_analyze | rows=5 |
| PASS | mesh-mcp.registry | registry endpoint is :3276 | http://127.0.0.1:3276/call |
| PASS | mesh-mcp.positive | mesh_analyze succeeds with a real answer | {"output":{"bodies":[{"face_count":12,"index":0,"is_volume":false,"vertex_count":8,"volume":null}]," |
| PASS | mesh-mcp.ripple | the call carries the captured block (usage ledger + jurisdiction gate + graph ripple) | ["usage ledger row recorded","jurisdiction gate applied","graph ripple emitted for the call","graph persisted: root 4409 |
| PASS | mesh-mcp.refusal | mesh_analyze bad input → clean success:false + reason (no 5xx, no crash) | no such file: /home/rebornbeat/Projects/Ozone-Studio/tools/mesh-mcp/data/e2e/does-not-exist.obj |
| PASS | mesh-mcp.s13 | S13 capture holds a row for mesh_analyze | rows=5 |
| PASS | document-mcp.registry | registry endpoint is :3282 | http://127.0.0.1:3282/call |
| PASS | document-mcp.positive | document_extract succeeds with a real answer | {"output":{"backend":"pdftotext","chars":38,"format":"pdf","path":"/tmp/e2e_fixtures/fixture_text.pd |
| PASS | document-mcp.ripple | the call carries the captured block (usage ledger + jurisdiction gate + graph ripple) | ["usage ledger row recorded","jurisdiction gate applied","graph ripple emitted for the call"] |
| PASS | document-mcp.refusal | document_extract bad input → clean success:false + reason (no 5xx, no crash) | not a readable file: /tmp/e2e_fixtures/none.pdf |
| PASS | document-mcp.s13 | S13 capture holds a row for document_extract | rows=8 |
