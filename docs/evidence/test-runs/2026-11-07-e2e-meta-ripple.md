# e2e-meta-ripple — 2026-11-07

host: http://127.0.0.1:50051 · duration 1226s · PASS 11 · FAIL 0 · SKIP 1

| result | id | test | evidence |
|---|---|---|---|
| PASS | MR0 | status route live: policy + per-loop counters present | {"assistant_min_gap_secs":300,"heartbeat_secs":21600,"min_gap_secs":600,"startup_catchup_secs":60} |
| PASS | MR1 | Workspace/Project (structure) creation does NOT wake the meta loops | meta_events 7->7 |
| PASS | MR1b | ...but the dispatcher did see the events | events_seen 153->155 |
| PASS | MR2 | a Task ripple is a META event (meta_events rose) | 7->8 task=1126 |
| PASS | MR2-i_loop | i_loop was woken by the meta ripple | 7->8 |
| PASS | MR2-meta_loop | meta_loop was woken by the meta ripple | 7->8 |
| PASS | MR2-assistant | assistant was woken by the meta ripple | 33->35 |
| PASS | MR3 | burst of 20 task events → ≥20 meta events seen | +20 |
| PASS | MR3-i_loop | i_loop: 20 events coalesce into ≤2 passes (min-gap 600s), not 20 | passes +1, wakes +20 |
| PASS | MR3-assistant | assistant: 20 events coalesce into ≤2 passes (min-gap 600s), not 20 | passes +1, wakes +40 |
| PASS | MR4 | no self-trigger runaway: an idle window produces ≤1 further assistant pass | passes +0, echoes ignored=25 |
| SKIP | MR5 | MethodologyGap ripple wakes the meta-loop (end-to-end) | needs a request that matches no methodology; covered by unit tests (ripple_class::tests) + emit site in amt.rs; add a gap-forcing orchestrate case |
