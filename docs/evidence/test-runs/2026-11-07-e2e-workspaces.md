# e2e-workspaces — 2026-11-07

host: http://127.0.0.1:50051 · duration 4s · PASS 5 · FAIL 0 · SKIP 0

| result | id | test | evidence |
|---|---|---|---|
| PASS | W1a | create workspace A and B | A=1004838470 B=1302013921 raw={"context":null,"error":null,"files":null,"project":null,"projects":null,"success":true,"workspace":{"created_at":179405 |
| PASS | W1b | a project in each workspace | pA=1793336142 pB=2144246208 |
| PASS | W1c | no cross-bleed: A lists only its project, B only its own |  |
| PASS | W1d | attach a markdown file to project A | {"directory_ref_id":null,"error":null,"file_ref":{"analysis_graph_id":null,"analyzed":false,"created_at":1794050522,"exists":true,"id":2794777233,"modality":"Te |
| PASS | W1e | project A holds notes.md only; project B holds main.rs + data.csv | A={"context":null,"error":null,"files":[{"id":2794777233,"modality":"Text","modified":1794050522,"name B={"context":null,"error":null,"files":[{"id":3397845997,"modality":"Code","modified":1794050522,"name |
