# e2e-keyword-program — 2026-11-08

host: http://127.0.0.1:50051 · duration 4s · PASS 9 · FAIL 0 · SKIP 1

| result | id | test | evidence |
|---|---|---|---|
| PASS | K1 | token keeps every exact match and drops the substring flood ('file': far fewer than scan) | {"flood":{"file":{"scan":1098,"token":216,"exact":1},"code":{"scan":1354,"token":1133,"exact":77},"review":{"scan":612,"token":625,"exact":14},"graph":{"scan":162,"token":422,"exact":7}},"exactKept":{"file":true,"code":true,"review":true,"graph":true}} |
| PASS | K2 | the ranked token search returns the same ORDER twice (deterministic v0 key) | 113 ids |
| PASS | K3 | a search appends a `search` hit event with its terms and returned ids | bytes 339480 -> 346386 |
| PASS | K4 | ModelStatus lists the prepared pretrained-finetune and embedder slots | [["pretrained-finetune","prepared"],["embedder","prepared"]] |
| PASS | K5 | TrainEmbedder / FineTunePretrained report success:false, available:false and what they require | {"emb":"TrainEmbedder is PREPARED, not runnable: it needs a labelled corpus large enough","ft":"FineTunePretrained is PREPARED, not runnable: it needs third-party pretrained we"} |
| PASS | K6a | TrainModel on an empty dataset trains nothing and says why (models untouched) | nothing trained: the hit log has no usable labelled groups yet |
| PASS | K6b | TrainModel on a labelled dataset trains, evaluates on the held-out groups, and registers encoder + ranker with the MEASURED decision | {"activated":false,"wins":0,"losses":0,"p":1,"reason":"does not beat the deterministic v0 ordering on held-out real groups"} |
| PASS | K7 | RankCandidates with no active model returns the deterministic v0 order (exact term first) with every candidate once | [2,1,3] |
| PASS | K8 | TraverseSemantic refuses honestly (success:false + pointer to RankCandidates), no fake empty success | TraverseSemantic has no pipeline-side index: use the RankCandidates action with host-supplied candidates (active encoder |
| SKIP | K9 | pipeline 47 accepts the {data, context} envelope | consciousness pipelines are internal meta, not callable via external contracts — verified by Stage 12 of a real walk |
