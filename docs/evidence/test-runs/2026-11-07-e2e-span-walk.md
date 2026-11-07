# e2e-span-walk — 2026-11-07

host: http://127.0.0.1:50051 · duration 3605s · PASS 3 · FAIL 4 · SKIP 0

| result | id | test | evidence |
|---|---|---|---|
| PASS | SW0 | code workspace + project created | ws=2127615338 proj=2318248211 |
| PASS | SW1 | both code files became modality graphs in the project | [45023,45024] |
| PASS | SW2 | direct span-out from the code graphs is entry-rooted (run_report/main → ingest → normalize → score) | ## ModalityGraph: /tmp/e2e_spanws/pipeline_app.py code graph (python, 13 lines, 2 functions, 0 classes) (id 45023) Keywo |
| FAIL | SW3 | the walk finished | {"state":"timeout","err":""} |
| FAIL | SW4 | step contexts carry span-out provenance (span-out:K items, T tokens, F fetched) | 0/0 steps; [] |
| FAIL | SW5 | the assembled step context holds the graph span-out text with the real functions |  |
| FAIL | SW6 | the delivered answer names the real call order (ingest_records → normalize_records → score_records) | {"state":"timeout","run_token":"run-1794053093231317443"} |
