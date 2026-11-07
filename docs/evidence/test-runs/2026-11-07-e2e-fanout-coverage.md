# e2e-fanout-coverage — 2026-11-07

host: http://127.0.0.1:50051 · duration 2s · PASS 17 · FAIL 0 · SKIP 13

| result | id | test | evidence |
|---|---|---|---|
| SKIP | CAP-audio | audio: 1/1 graph(s) captured NO edges (capture gap, not a fan-out gap) | tracked |
| PASS | COV-bci | bci: ≥95% of edges classified into a fan-out class | other=0/1 |
| PASS | COV-biology | biology: ≥95% of edges classified into a fan-out class | other=0/2 |
| PASS | COV-cad | cad: ≥95% of edges classified into a fan-out class | other=0/2 |
| PASS | COV-chemistry | chemistry: ≥95% of edges classified into a fan-out class | other=0/6 |
| PASS | COV-code | code: ≥95% of edges classified into a fan-out class | other=0/295 |
| SKIP | CAP-code | code: 1/50 graph(s) captured NO edges (capture gap, not a fan-out gap) | tracked |
| SKIP | CAP-control | control: 1/1 graph(s) captured NO edges (capture gap, not a fan-out gap) | tracked |
| PASS | COV-dna | dna: ≥95% of edges classified into a fan-out class | other=0/2 |
| SKIP | CAP-dna | dna: 1/2 graph(s) captured NO edges (capture gap, not a fan-out gap) | tracked |
| PASS | COV-eeg | eeg: ≥95% of edges classified into a fan-out class | other=0/5 |
| SKIP | CAP-electromagnetic | electromagnetic: 1/1 graph(s) captured NO edges (capture gap, not a fan-out gap) | tracked |
| PASS | COV-geospatial | geospatial: ≥95% of edges classified into a fan-out class | other=0/1 |
| SKIP | CAP-haptic | haptic: 1/1 graph(s) captured NO edges (capture gap, not a fan-out gap) | tracked |
| SKIP | CAP-hyperspectral | hyperspectral: 1/1 graph(s) captured NO edges (capture gap, not a fan-out gap) | tracked |
| PASS | COV-image | image: ≥95% of edges classified into a fan-out class | other=0/63 |
| PASS | COV-imu | imu: ≥95% of edges classified into a fan-out class | other=0/3 |
| PASS | COV-kinematics | kinematics: ≥95% of edges classified into a fan-out class | other=0/21 |
| PASS | COV-math | math: ≥95% of edges classified into a fan-out class | other=0/25 |
| SKIP | CAP-network | network: 1/1 graph(s) captured NO edges (capture gap, not a fan-out gap) | tracked |
| SKIP | CAP-proteomics | proteomics: 1/1 graph(s) captured NO edges (capture gap, not a fan-out gap) | tracked |
| SKIP | CAP-radar | radar: 1/1 graph(s) captured NO edges (capture gap, not a fan-out gap) | tracked |
| PASS | COV-sonar | sonar: ≥95% of edges classified into a fan-out class | other=0/1 |
| PASS | COV-sound | sound: ≥95% of edges classified into a fan-out class | other=0/2 |
| PASS | COV-text | text: ≥95% of edges classified into a fan-out class | other=0/1048 |
| SKIP | CAP-text | text: 53/179 graph(s) captured NO edges (capture gap, not a fan-out gap) | tracked |
| PASS | COV-thermal | thermal: ≥95% of edges classified into a fan-out class | other=0/5 |
| SKIP | CAP-video | video: 1/1 graph(s) captured NO edges (capture gap, not a fan-out gap) | tracked |
| SKIP | COV-orphans | 42 graph file(s) of 309 have NO owning container (data hygiene finding: leftovers, mostly pre-existing code graphs) | tracked — not a fan-out defect |
| PASS | COV-all | every graph that HAS an owning container is fanned out (fanout evidence present) | {} |
