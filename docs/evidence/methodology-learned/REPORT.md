# Methodologies from lessons learned — report (2026-11-12)

Commit 5598f3a (docs only; 30 files here). Nothing applied: apply.mjs not run, EU and local store untouched. Written by the parent from the fork's output.

27 drafts, same JSON format as the existing 30, each with an `evidence` list (file + line/section); 01–30 not duplicated.
New 31–49: no-skip; acceptance is readability; invariants by construction (step index by position); never mint an identifier; durable ordered writes; resource floors pause never skip; link first, walk adds intelligence; history chunked/verified/reversible; live data changes (look, copy, check, apply one, verify, restore); hand validation while settlement paused; report at the level done (evidence-first status, fork-report handling); coordinate agents via shared context; every entry point names its credential; review every call site against the gate; jurisdiction per-instance policy; MCP agents participate additively; host-called pipelines accept envelope and flat input; measure and batch our own judgment volume; generated facts with a drift check.
Add-only updates 50–57: work-order expansion/completion gates; folding redundant components into the host + keeping agent integrations current; no mocks; stop tuning a working parameter; paired-device sync vs P2P marketplace; dead-code removal lessons (the remove-only-on-superseded rule is already draft 17; 55 adds what the first real pass taught); protocol constants + backoff as spacing; retiring a persisted field.

Overlaps flagged (each can stand alone or fold in as an update): 31 vs 12/11; 35 vs built-in method_22/23; 39 vs 15; 40 vs 07; 41 vs built-in method_33 (recommend fold into 33).
Withheld, needs the operator: (1) quota gating for background actors (2026-11-08 finding vs draft 23 and the operator's no-quota-gating ruling) — left out of draft 48, which keeps its three uncontested rules; (2) MCP as a model (not built, no methodology yet); (3) legal specifics — draft 45 is engineering input, no legal claim.
Checks: lint clean on all drafts. `requires` only on tools believed registered on the EU (file_history, file_diff, code_callgraph), all optional; the EU store was not readable, so those names and name collisions are unconfirmed until the dry run.

Apply order (APPLY_ORDER.md): set 1 (01–30) → 31–49 → updates 50–57 in a second apply.mjs run (update targets resolve from a name snapshot taken at run start; apply 53 after draft 28) → hand-validate new keywords after each batch (new containers' terms register PENDING; strict search ignores PENDING, so none is findable until validated).
Note: git-mcp tool names in draft `requires` may change with the history rewrite (history/diff/read_version, aliases file_*); re-check at dry run.
