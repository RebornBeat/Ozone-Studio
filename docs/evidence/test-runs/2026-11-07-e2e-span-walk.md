# e2e-span-walk — 2026-11-07

host: http://127.0.0.1:50051 · duration 1203s · PASS 4 · FAIL 3 · SKIP 0

| result | id | test | evidence |
|---|---|---|---|
| PASS | SW0 | code workspace + project created | ws=5768392293 proj=5935060183 |
| PASS | SW1 | both code files became modality graphs in the project | [44973,44974] |
| PASS | SW2 | direct span-out from the code graphs is entry-rooted (run_report/main → ingest → normalize → score) | ## ModalityGraph: /tmp/e2e_spanws/pipeline_app.py code graph (python, 13 lines, 2 functions, 0 classes) (id 44973) Keywo |
| PASS | SW3 | the walk finished | {"state":"finished","task":1152,"err":"PAUSED: task 1152 paused at step 0: watchdog timeout after 300s — pipeline 21 call hung. It was not continued. Resume it"} |
| FAIL | SW4 | step contexts carry span-out provenance (span-out:K items, T tokens, F fetched) | 0/0 steps; [] |
| FAIL | SW5 | the assembled step context holds the graph span-out text with the real functions |  |
| FAIL | SW6 | the delivered answer names the real call order (ingest_records → normalize_records → score_records) | {"accepted":false,"amt_summary":null,"attempt_trail":["openrouter/free success 5810ms -> usable: primary answered","openrouter/free success 6214ms -> usable: primary answered","openrouter/free success |
