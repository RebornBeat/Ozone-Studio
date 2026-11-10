# FORK V2 — manual keyword validation (Blueprint / Pipeline / ModalityGraph / Category / McpEntity), EU, 2026-11-10

Judge: the assistant, per the operator's instruction ("validate all unvalidated yourself first, remove any you deem not worthy"). Settlement stays paused (`OZONE_KEYWORD_SETTLE=off`, untouched). No source file edited, nothing committed, no build/test.

## Result (115 containers, 1,152 unvalidated non-marker terms)
| type | containers | terms | validated (yes) | removed (no) | left unvalidated (unsure) |
|---|---|---|---|---|---|
| Category | 12 | 33 | 14 | 19 | 0 |
| Pipeline | 55 | 263 | 148 | 115 | 0 |
| Blueprint | 26 | 603 | 168 | 433 | 2 (40212: `error handling`, `data validation`) |
| ModalityGraph | 22 | 253 | 68 | 184 | 1 (2131: `scene graph`) |
| **Total** | **115** | **1,152** | **398** | **751** | **3** |

Independent re-scan of the EU store after the run (read-only): Pipeline 148 validated / 0 unvalidated; Category 14 / 0; Blueprint 197 validated / 2 unvalidated; ModalityGraph 91 validated / 1 unvalidated; no term is both removed and still present; `/health` 200, service active.
Before/after diff of every touched container: only `context.keywords` and `context.keyword_meta` changed (metadata/storage/hints/other context identical apart from `updated_at`).

## McpEntity — NOT touched (special case)
All 862 McpEntity containers with unvalidated terms (1,974 terms) are UI-ingest bookkeeping rows (keywords such as `uidocument_ingest`, `Element`, `StyleRule`, ...). There are **no non-ingest McpEntity containers with unvalidated terms**, so nothing from that type was judged. They remain untouched pending the tag-namespacing migration. Distinct tags by count (the first 14 are the actual ingest kinds; the long tail below them are zero-shot expansion junk already stored on those rows — e.g. `Google Inc.`, `Alphabet`, `MCP entity Brazil`, `gb_Lb`, `McpEntity .gb_Ta.gb_eb` — worth a separate, reviewed cleanup after namespacing):
`uidocument_ingest` 842, `Element` 409, `StyleRule` 167, `Link` 78, `ScriptRef` 70, `Field` 63, `Media` 14, `UiDocument` 12, `style rule` 11, `MCP entity` 10, `document ingestion` 10, `Shape` 9, `StyleSheetRef` 7, `Form` 7
Total distinct unvalidated tags on these rows: 238. Full list: `targets.json` -> `ingest_tags`.

## Method and evidence read
- Category: body is only `Auto-created category for topic: <name>`; the name/topic is the evidence. Own topic and close name forms = yes; rewordings, hallucinated expansions (e.g. `alternative minimum tax`, `amt expand liquidity`) = no.
- Pipeline: container name + folder (`pipelines/<category>/<folder>`; no body file exists). The exact pipeline name and concept words of it = yes; `pipeline`, folder labels (`core`, `general`) and generic role/verb fragments (`tab`, `check`, `return`, `update`, `create`, `register`, `fetch`, `manager`, `loader`, ...) = no.
- Blueprint: for the 19 built-ins no body exists on the EU, so evidence = the bootstrap index's declared keywords/input/output types + name + description + steps of the converted `assets/blueprints/bp_*.json`; for the 7 walk blueprints = their real bodies. Declared routing keywords, the blueprint name and subjects its description/steps state = yes; type labels (`text`, `text graph`, `file_reference`), question-stem function words (`what is`, `how does`), garbled error-message fragments, and the many rewordings/hallucinated expansions (`data analytics consulting`, `new hire onboarding`, `knowledge graph` ...) = no.
- ModalityGraph: graph file nodes (Keyword/Topic/Entity/Image/Region/Text nodes) are the evidence. Terms present as nodes and naming real subject matter = yes; graph-type labels, word/sentence counts, ids, event-kind labels (`coordinationreceived`), error-message fragments, variant spellings and translations of `image` = no.
- Asymmetric: unsure = leave (3 terms). No scores. Reasons for `no` are recorded in `decisions.jsonl`; for the bulk categories they are category-level sentences (e.g. "near-duplicate rewording or unsupported expansion of a kept term"), not a unique sentence per term.

## Apply mechanics
`apply.mjs` (run on the EU): per container GetContainer -> saved to `before/container_<id>.json` -> keywords/keyword_meta rewritten exactly like `tools/keyword-measure/backlog_cleanup.mjs` (yes: `validated:true` keeping existing meta/hits; no: removed from `keywords`, `keyword_meta[term].validated=false` so it is never re-proposed) -> `UpdateContainer` -> re-read (`after/container_<id>.json`) and verified term by term; stale-decision guard (acts only on currently-unvalidated terms); idempotent re-run (skips already-applied containers, never overwrites a before-state). Pilot: 40071 (Category) and 10002 (Pipeline) verified exactly, then all 113 others in one run, 0 anomalies, health 200.
`TermMeta` has no field for a review label, so none was recorded in the store; provenance is this directory.

## Restore (any term)
`node /opt/ozone-build/tools/keyword-measure/restore_term.mjs <container_id> "<term>"` restores a term as pending (validated:null) with its counters; to flip a `yes` back to pending set `validated` to null through UpdateContainer, or restore the whole container's context from `before/container_<id>.json` (`local_state.context`). Every decision with the previous meta is in `decisions.jsonl`.

## Surprises / not verified
- Every Pipeline container's keywords were only name tokens + `pipeline` + folder; none had been hit-validated.
- McpEntity (EU re-scan): 36 validated terms already existed and 1,974 unvalidated remain — all on ingest rows, untouched.
- Not verified: the local (down) host; hit counters were not consulted per term (stored hotness untouched); the host's behaviour for the removed terms on the next settlement/gate run was not exercised (settlement is paused).
- Files: `decisions.py` (the judgments), `decisions.json/.jsonl`, `apply.mjs`, `targets.json`, `evidence_raw.json`, `before/` (115), `after/` (115), `applied_ledger.jsonl` (same as decisions.jsonl).
