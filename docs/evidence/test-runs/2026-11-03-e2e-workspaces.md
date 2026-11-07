# e2e-workspaces — 2026-11-03

host: http://127.0.0.1:50051 · duration 3s · PASS 5 · FAIL 0 · SKIP 0

| result | id | test | evidence |
|---|---|---|---|
| PASS | W1a | create workspace A and B | A=8173338515 B=8371694289 raw={"context":null,"error":null,"files":null,"project":null,"projects":null,"success":true,"workspace":{"created_at":179370 |
| PASS | W1b | a project in each workspace | pA=8645316538 pB=8806762732 |
| PASS | W1c | no cross-bleed: A lists only its project, B only its own |  |
| PASS | W1d | attach a markdown file to project A | {"directory_ref_id":null,"error":null,"file_ref":{"analysis_graph_id":null,"analyzed":false,"created_at":1793706829,"exists":true,"id":9309408349,"modality":"Te |
| PASS | W1e | project A holds notes.md only; project B holds main.rs + data.csv | A={"context":null,"error":null,"files":[{"id":9309408349,"modality":"Text","modified":1793706829,"name B={"context":null,"error":null,"files":[{"id":9919757863,"modality":"Code","modified":1793706829,"name |
