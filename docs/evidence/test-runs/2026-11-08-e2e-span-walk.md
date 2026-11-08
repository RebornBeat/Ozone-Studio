# e2e-span-walk — 2026-11-08

host: http://127.0.0.1:50051 · duration 2000s · PASS 5 · FAIL 2 · SKIP 0

| result | id | test | evidence |
|---|---|---|---|
| PASS | SW0 | code workspace + project created | ws=3107294278 proj=3582646232 |
| PASS | SW1 | both code files became modality graphs in the project | [45086,45087] |
| PASS | SW2 | direct span-out from the code graphs is entry-rooted (run_report/main → ingest → normalize → score) | ## ModalityGraph: /tmp/e2e_spanws/pipeline_app.py code graph (python, 13 lines, 2 functions, 0 classes) (id 45086) Keywo |
| PASS | SW3 | the walk finished | {"state":"finished","task":1155,"err":""} |
| FAIL | SW4 | step contexts carry span-out provenance (span-out:K items, T tokens, F fetched) | 0/0 steps; [] |
| FAIL | SW5 | the assembled step context holds the graph span-out text with the real functions |  |
| PASS | SW6 | the delivered answer names the real call order (ingest_records → normalize_records → score_records) | {"accepted":false,"amt_summary":{"branch_count":44,"branches":[{"confidence":1,"content":"explain the order in which run_report calls its helper functions and what each one does.","depth":0,"id":47,"r |
