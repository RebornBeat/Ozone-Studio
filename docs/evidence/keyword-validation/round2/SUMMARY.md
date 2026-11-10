# FORK A — round 2: clearing keywords by hand + merge + strict-mode measurement (EU, 2026-11-10)
Judge: the assistant. Settlement stays paused. No source edits, nothing built/tested/committed. All writes through the host API (UpdateContainer), exactly the apply.mjs of the earlier rounds (copy in this dir); before-state of every container saved first (`applied/before`, `applied_mcp/before`), after-state re-read and verified (`applied*/after`), health 200 between batches, pilots first (40311 + 40226; 1528 + 1684), 0 anomalies.

## 1. The terms the earlier rounds left unvalidated (Methodology 335, Blueprint 2, ModalityGraph 1 = 338 terms, 71 containers) — `decisions.json`
yes (validated:true) 104 · no (removed, keyword_meta validated:false) 158 · still unresolved (untouched) 76, each with a one-line reason in the ledger. Hit guard: no removed term has kept/applied/packed/human > 0 (checked before applying and again from the saved before-state). Identifier-shaped terms with kept hits (`container 40211` x3 forms) were NOT removed and NOT validated. Includes the merged canonicals 40305/40350 and the duplicates 40252/40348.
Decision rules used (asymmetric): YES only when the body/topic supports the term and it is not a near-duplicate of a kept term; NO only with a stated reason (generic word, fragment, near-duplicate respelling, a different discipline, belongs to a sibling methodology, templated family form); doubt = leave.

## 2. Other gated types
Fresh full scan of /opt/ozone-studio/zsei_data/local: Project, Workspace, FileReference containers carry NO non-marker keywords (nothing to judge). Blueprint/Pipeline/Category/ModalityGraph: 3 -> 1 left (2131 `scene graph`, graph body empty on disk).
McpEntity: the 862 UI-ingest rows' bookkeeping tags (uidocument_ingest, Element, StyleRule, Link, ScriptRef, Field, Media, UiDocument, Form, Shape, StyleSheet, StyleSheetRef) were NOT touched (namespaced by the backfill after deploy). Only 34 of those rows carried expansion terms beyond the tags (289 occurrences, 226 distinct terms), judged against each row's own name/body (`decisions_mcp.json`): yes 17 (terms present in the node's own content: `html`, `head`, `Graphic`, `Google`, `CSS selector`, ...) · no 260 (MCP/entity/ingest-mechanism phrases, restatements of the node kind, company names and guessed functions not in the node's content, e.g. `MCP entity Brazil`, `Google Inc.`, `Alphabet`) · leave 12. Hit guard clean.

## 3. Merge as needed — `docs/evidence/methodology-merge-2/`
Scanned all 98 Methodology containers (name tokens, keyword Jaccard, rule/heuristic overlap): ONE new candidate, 40394/40396 (same name, different rules). Merged additively into 40396 (+2 rules, +1 heuristic, +2 validated keywords; restatements not moved; `requires` left empty on purpose). 40394 not retired (only hard DeleteContainer exists). Earlier pairs unchanged.

## 4. Final state per gated type (non-marker keywords; `scan_after.json`)
| type | validated | unvalidated |
|---|---|---|
| Methodology | 1,272 | 75 (the 'leave' terms; 1 of the 76 is on the ModalityGraph) |
| Blueprint | 197 | 0 |
| Pipeline | 148 | 0 |
| Category | 14 | 0 |
| ModalityGraph | 91 | 1 |
| McpEntity | 53 | 1,697 (bookkeeping tags 1,685 + 12 'leave') |

## 5. Strict search mode measured, NOT flipped (`strict_recall.py`, `strict_recall_out.json`; logged `search` events in keyword_hits.jsonl, all 1,349 query-term instances)
Strict = a container's keywords count only if marker or validated:true (src/zsei/search.rs, applies to EVERY container type); names and topics unaffected.
- Queries that find at least one CONTENT container: now 643/706 (orchestrate), strict 643/706 — NO content query is lost. `none`-origin 576/609 and `other` 33/34 unchanged.
- Any-type: 17 orchestrate queries (one term, `profilesprofilestart`, found only a Derived container) would find nothing.
- Result volume (content containers returned): orchestrate 7,684 -> 6,793 (-12%), none-origin 4,999 -> 2,156 (-57%), other 3,013 -> 3,010. Strict mainly removes the noise; it does not remove the ability to find content.
- STRUCTURAL PROBLEM (not a recall issue): strict applies to types that are never keyword-gated, so their keywords can never become validated and would all stop matching: JurisdictionRuleSet 877 keyword slots in 132 containers (these are region/ISO routing keys), Experience 63 containers, Modality 9, ChatTurn 10, ChatSession, every *Root container (~45), McpEntity ingest rows 837. Strict mode must be limited to the gated types (or ungated types exempt) before it is flipped, or those lookups break. Source change needed (keyword_program::searchable_keywords / search.rs): NOT made here.

## Restore recipe
Whole container: UpdateContainer with `context` from `applied/before/container_<id>.json` (or `applied_mcp/before/...`). One term: `node /opt/ozone-build/tools/keyword-measure/restore_term.mjs <container_id> "<term>"` (restores as pending, counters kept). Merge: see `docs/evidence/methodology-merge-2/REPORT.md`.

## Not verified
Local store (its host is down). No per-term hit counters were consulted beyond the kept/applied/packed/human guard. `validated:true` means reviewed by the assistant, not the strength-N gate (TermMeta has no validator field; the ledger is the provenance). The strict-mode comparison approximates the host's matching (whole-token subset + exact term) as expansion_breakdown.py does; ranking effects are not modelled. 40394 is still gathered alongside 40396. The 75+1 unresolved methodology/graph terms and the 12 McpEntity 'leave' terms were left on purpose.
