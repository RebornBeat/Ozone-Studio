# Operator decision log (2026-11-11 session)

Quotes are the operator's words; "you decide" items were delegated and decided by Claude Code.

| Decision | Source / basis |
|---|---|
| AMT container rewritten as a new version after each expansion round | "Okay rewrite as new version yes" |
| Unique step_index enforced at plan creation | "enforce correct" |
| No fixed caps limiting traversal; chunk size derived from the reading model's window | "chunking is the only thing there is that has caps ... anything against this yes should be removed or updated or replaced" |
| Imported peer methodologies get no recheck at accept (marketplace validates before download) | "no placeholder imported peer methodology check ... done once validated" |
| Memory floor pauses and resumes, never skips or stops | "ensure that we do not skip it pauses etc but no skip or stop pause" |
| UI forged demo key removed | delegated, decided |
| dry_run_first: no superseder identified, left for review | delegated, decided |
| Duplicates 40252/40394: repoint citers then hard-delete (scripted, applied after build gate) | delegated, decided; 40348 already deleted |
| EU settlement stays paused (operator drop-in); strict search ON | delegated, decided |
| OZONE_KEYWORD_IGNORE_EXPANSION and OZONE_KEYWORD_STOPLIST stay OFF (measurement/baseline toggles) | delegated, decided |
| Seven superseded pub dead-code items removed only if superseded, captured, nothing dropped | "you decide if superseeded and ensured and captured and nothing dropped then remove" |
| Cross-file call resolution as a deterministic structural layer; never interferes with semantic layers | delegated, decided |
| Backup branch backup-2026-11-11-pre-build created locally; push only after build gate + operator go | delegated, decided |
| MCP agents participate in walks; an MCP call can be the prompt | operator clarification |
| All switches default ON in code; local opts out in own config | operator ruling |
| Tool creation automatic on identified need; findings published | operator ruling (earlier) |
| Completed is final; rerun = new instance | operator ruling (earlier) |
| Text graph failure: no minted ids; record failure, continue from chunked text, retry child item gates completion | delegated, decided |
| Gap checklists and queued re-expansion candidates do not gate; failed child parks visibly with operator retry/cancel | delegated, decided |

## Addendum 2026-11-11 (no-skip sweep)
| Decision | Source / basis |
|---|---|
| step_index never fails/replans/pauses: assigned by position once at population; normalize deterministically | "why would a step index ever repeat … not a fail or retry … just change step index" |
| THERE IS NO SKIP: failures are fixed, or the task pauses and retries (gates await, never fail open, never silently drop) | "we don't record failures and move on all failures are to be fixed we don't skip anything thats a doctrine" |
| Acceptance == readability: only extensions with a reader link; rejected at link time with the exact reason; no "unreadable" state | coordinator relay of operator correction: "there should be NO unreadable files, because only accepted modalities/extensions link at all" |
| Malformed file of an accepted type = link-time validation failure; if it reaches the worker it is a real failure → pause + retry | same |
| Sensor modalities without self-describing files: not accepted (follow-up: pipelines accept UNSPECIFIED parameters) | decided in the sweep; reversible |

## Addendum 2026-11-11 (later)
| Decision | Basis |
|---|---|
| Delete the 7 redundant meta-loop drafts (40222, 40226, 40262, 40264, 40268, 40395, 40126) ONLY once canonical bodies 40396 and 40125 are live (rules applied) and the same citation/coverage checks as the 40252/40394 pair pass; before-copies + restore first | operator: "yes only once canonical bodies are live" |
| Step index assigned centrally by position at population; duplicates normalized, never replanned or failed | operator: "work should be done to prevent this error not a fail or retry" |
| No skip: every failure is fixed or the task pauses and retries; nothing recorded-and-moved-on | operator doctrine restated |
| No "unreadable" files: acceptance == readability; unreadable extensions rejected at link time with reason; corrupted files fail validation at link time | operator: "there shouldn't be any to link it has to be an accepted modality" |

## Addendum 2026-11-11 (sensor modalities)
| Decision | Basis |
|---|---|
| Sensor-modality raw-input path DEFERRED: sensor extensions (imu, iq, radar, sonar, hyperspectral, haptic, bci, pts/e57, etc.) stay rejected at link time with the exact reason; acceptance == readability stays enforced. Not forgotten: tracked as open in CHECKLIST (no-skip-sweep / modality readers) | operator: "sensor modalities no continue with all minus this but yes ensure captured" |
| True test totals captured by static count (not run): 611 tests in 101 files (src 287/58, tests 5/2, shared 24/4, modality pipelines 267/31, general pipelines 17/5, shared pipelines 11/1); passing totals only from a real run | operator: "capture true test totals" |
