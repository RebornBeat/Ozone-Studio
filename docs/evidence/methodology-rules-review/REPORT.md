# Rules review by hand — all 98 EU methodologies (2026-11-10, READ-ONLY; nothing applied)

Method: every Methodology container on the EU (98) was read through its body (data dir, else app root — 48 live under the app root); every decision rule (condition + action/outcome) and heuristic (when_to_apply + description) with real text was read in full (263 decision rules, 210 heuristics; 16 `requires` entries). Providers were taken from the EU registry (`zsei_data/mcp_tool_registry.json`, grouped by endpoint exactly as `provider_for_endpoint` does): `security-mcp`, `browser-farm-mcp`, `document-mcp`, `p2p-bridge-mcp`. Duplicates were found by token overlap (>= 0.33 across methodologies, >= 0.5 within one) and then judged by reading. NOTHING was changed on the EU.
Machine-readable: `fixes.json` (82 rule-level findings: methodology id, exact rule text, action `retire` | `rewrite` | `review`, class, reason) and `consolidations.json` (10 methodology-level clusters).
`retire` entries are the ones the new `remove_decision_rules` / `remove_heuristics` Update can apply later (exact text, versioned, refuses to leave a methodology empty); `rewrite` = retire + add in ONE Update; `review` needs a decision first.

## Result
21 of 98 methodologies have findings; 77 have none (5 of those are the `steps`-shaped 1000–1004, which have no rules at all — outside this review).

| class | findings | action |
|---|---|---|
| (a) duplicate / near-duplicate rule inside one methodology | 13 | 12 retire, 1 review |
| (a) duplicate methodologies (cross) | 10 clusters | consolidations.json (2 already merged) |
| (b) rule bound to ONE incident (literal container ids) | 16 | rewrite |
| (b) rules contradict each other | 4 (2 pairs) | review |
| (b) name collision: rules are about a different meaning of the name | 8 (+1 off-subject heuristic) | review |
| (c) fixed numeric cap with no measurement | 2 | retire |
| (c) vague / untestable action | 8 | review |
| (d) `requires` names a provider not registered on the EU | 10 rules (16 entries) | review |
| (d) `requires` names a registered TOOL with kind `provider` | 6 | rewrite |
| (d) text names a tool/provider that does not exist | 14 | review |

## (a) Duplicates
- **Merged pairs (done earlier today):** 40305←40252 and 40350←40348. The canonicals now hold rules that exist twice across methodologies (40350 R4/R5/R6/H2 = 40348 R0/R1/R2/H0; 40305 R5/R6/H2/H3 = 40252 R1/R3/H0/H1) because the duplicates are still stored (only a hard `DeleteContainer` exists). Inside 40350 the merge also put R2 ("set resource limits and isolation boundaries") beside R5 ("limit its privileges, resources, and network access") — R2 is subsumed: **retire R2**; H0 ("changed by hand → rebuild") repeats R0 ("build and deploy a new image"): **retire H0**. 40305 R4 overlaps R0's "recorded history": review.
- **Inside one meta-loop draft:** 40222 H0/H1 = R0/R1; 40396 H1 = R2 and H0 restates R0+R1; 40264 H3 = R3; 40126 H0/H1/H2/H4 restate R0/R1+R3/R2/R5; 40262 H0 = R0 — all **retire** (see fixes.json).
- **Cross-methodology clusters** (consolidations.json): input validation (40317 ← 40382, related 40312), error handling (40314 ← 40381, related 40321/40389), deduplication (40313 ← 40384), hierarchy (40315 ← 40385), testing (40322 ← 40388; 40326 Verification is a different concern and stays), registry (40351 ← 40391), capability declarations (40349, related 40383; 40352 is a different sense, see (b)), container (40350, related 40386), configuration (40305). Each "&" methodology (40381–40385, 40388, 40391) is the same topic with extra rules, so the additions should move into the plain one the way the two pairs were merged.
- **The AMT re-expansion / farm-start incident cluster:** eleven drafts (40084, 40125, 40126, 40222, 40226, 40262, 40264, 40268, 40394, 40395, 40396) around ONE incident overlap each other and should become ONE generalized methodology (40396 is the only one built from registered tools).

## (b) Not applicable whenever gathered (the operator's "all rules apply" test)
- **Literal incident ids (16 rules):** 40222 (container 3645), 40125 (40047), 40126 (e.g. 40090), 40226, 40262 / 40268 (40211), 40396 (3644). They hold only when that exact container is the subject.
- **Contradictions:** 40125 R3 (version gap > one minor → abort, require INITIAL_BUILD) vs its H1 (the same gap → full re-expansion from scratch); 40084 R0 (retry a `[object ob]` response if the endpoint answers validly) vs its H0 (any `[object ob]` body → abort, mark failed, no retry). A sound methodology cannot hold both.
- **Name collisions:** 40352 Capability Management — every rule is about an ORGANIZATION's strategic capabilities (objectives, differentiators, investment), so it is gathered for MCP/tool "capability" requests where none applies; 40320 Mapping — every rule is cartographic (scale, projection, legend), gathered for graph/"mapping" requests. Fix: rename/redescribe (the description is what the gather step matches on), not rule edits. Also 30024 H0 (a CSS-specific heuristic inside a general reconciliation methodology).
- **Broad but sound:** 40321 Resilience Engineering, 40325 System Architecture and the rest of the regenerated principles-only set keep a single concern; no rule there failed the test by reading.

## (c) Vague / untestable / capped
- **Fixed caps:** 40264 R4 and H2 abort at "6 hierarchy levels" with no measurement behind the number (operator doctrine: no fixed limits) — **retire both**. Other numeric thresholds are retry/backoff or timing guidance (40084: 5 s / 3 probes; 40126 H5: three retries; 40222 R1: 1 s, 2 s, 4 s; 40396 R2: 2 s; 30017 R1: ~1 GB) — noted, not flagged for retirement because they are operational guidance for the agent, not a system limit.
- **Vague actions (8):** 40352 H0 ("question whether it should be maintained"), 40352 H1 ("identify what is needed"), 40321 H1, 40383 H0, 40315 H1, 40382 H0, 40311 H0, 40305 H1 — the action asks for a judgment without saying what to do or how to know it happened.
- **Placeholders:** none — no "..." / "…" / empty rule survived in any of the 93 methodologies with rules.

## (d) Providers and tools
- **Invented providers (not registered on the EU):** `rollback-mcp` (40268 ×2), `reexpansion-mcp` (40395, 40126), `container-registry-mcp` (40262, 40394), `build-mcp`, `amt-hierarchy-mcp`, `validation-mcp`, `farm-profile-mcp` (all 40126). Each walk that gathers them records a missing capability (that is the `capability_gaps.json` row "AMT Re-Expansion Initial Build Methodology rule 0" on the EU). Decide per case: a real gap to build/acquire, or drop the requirement.
- **Registered tools written as providers (6, 40396 R0–R5):** `farm_status`, `farm_profiles`, `farm_profile_stop`, `farm_state_get`, `farm_profile`, `farm_state_set` are TOOLS of provider `browser-farm-mcp`; with `kind: provider` the binding looks for a provider of that name, never finds one, and reports a FALSE missing capability on every walk. Fix: `kind: "tool"` (resolves to its provider) — `rewrite` (remove + add in one Update).
- **Names in rule text that exist nowhere:** `amt-hierarchy-tool-v2`, `amt-hierarchy-tool` (40264), `amt_reexpansion_coordination_received`, `amt_reexpansion_container` (40226), plus the invented providers above (14 rules). 40264 H0 also invents an id format (`^[A-Z]+-[A-Z]+-\d{5}$`).

## Applying later
`fixes.json` → for each `retire`: `Update {remove_decision_rules|remove_heuristics: [condition]}` (after the crate is rebuilt). Note: blueprint citations `m<id>:r<n>` follow POSITIONS, so a retirement shifts later rules' refs; stored plans that cite them are re-audited by the citation gate (missing refs are removed). `review` items need your decision first.
