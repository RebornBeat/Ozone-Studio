# e2e-span-walk — 2026-11-08

host: http://127.0.0.1:50051 · duration 3608s · PASS 5 · FAIL 5 · SKIP 0

| result | id | test | evidence |
|---|---|---|---|
| PASS | SW0 | code workspace + project created | ws=965608894 proj=1151957442 |
| PASS | SW1 | both code files became modality graphs in the project | [45333,45334] |
| PASS | SW1b | decoy twin graphs created in a different workspace | [45337,45338] |
| PASS | SW1c | SCOPED span-out (project_id) never reaches the decoy project's graphs and finds the project's own | own=45333,45334 decoyHit= out_of_scope=2 |
| PASS | SW2 | direct span-out from the code graphs is entry-rooted (run_report/main → ingest → normalize → score) | ## ModalityGraph: /tmp/e2e_spanws/pipeline_app.py code graph (python, 13 lines, 2 functions, 0 classes) (id 45333) Keywo |
| FAIL | SW3 | the walk finished | {"state":"timeout","err":""} |
| FAIL | SW4 | step contexts carry span-out provenance (span-out:K items, T tokens, F fetched) | 0/0 steps; [] |
| FAIL | SW5 | the assembled step context holds the graph span-out text with the real functions |  |
| FAIL | SW7 | the walk's step contexts hold ONLY this project's code graphs (no decoy twin, no graphs from earlier runs) | graphs seen in contexts:  |
| FAIL | SW6 | the delivered answer names the real call order (ingest_records → normalize_records → score_records) |  |
