# e2e-clock-project — 2026-11-03

host: http://127.0.0.1:50051 · duration 1802s · PASS 0 · FAIL 3 · SKIP 0

| result | id | test | evidence |
|---|---|---|---|
| FAIL | CLK1 | Ozone walk finished (not lost) | state=finished 1802s PAUSED: task 992 paused at step 0: watchdog timeout after 1800s — pipeline 100 invocation hung. The run failed before any step executed; nothing was continued.  |
| FAIL | CLK2 | response cites a Blender output path (.glb/.stl/.png) |  |
| FAIL | CLK3 | response cites a CAD output path (.step) |  |
