# UN-level elevation candidates (SUGGESTIONS ONLY — NOTHING APPLIED)

Every rule added to `global.json` in this batch is action `Log`. Elevation is an operator policy act. The lists below are suggestions for the operator to consider, with one line of reasoning each. Condition keywords are in quotes. Sources were retrieved 2026-10-27/28; some snippets come from search summaries, so re-verify before relying on them.

## Strongest candidates among the new rules
- "genocide" -> Block/Warn: peremptory, universally criminalized; prompts on this topic rarely have a benign operational purpose beyond education (false positives likely on history/news, so Warn may fit better than Block).
- "child pornography" / "child sexual abuse material" -> Block: the OPSC defines the offence; it aligns with the existing CRC Art. 34 Block rule on "exploit".
- "human trafficking" -> RequireConfirmation: Palermo Protocol defines it; the term is also common in legitimate research/NGO use.
- "biological weapons" / "chemical weapons" -> Warn or RequireConfirmation: BWC/CWC prohibit development and stockpiling outright; discussion of safety/history is common, so Block would over-trigger.
- "nuclear weapons" -> Warn: NPT context; heavy everyday news/education usage argues against Block.
- "child soldiers" -> RequireConfirmation: OPAC prohibits recruitment under 18 by armed groups; mostly advocacy context.
- "war crimes" / "crimes against humanity" -> Warn: Rome Statute offences; common in journalism/legal work.
- "torture" -> Warn: CAT Art. 2 has no exceptions; same news/legal false-positive profile.
- "terrorist financing" -> RequireConfirmation: UNSC 1373 is binding on all States; actionable requests warrant a human check.
- "sanctions" -> Warn: UNSC regimes change often; flagging helps prompt a current-list check.
- "child labour" / "forced labour" -> Warn: ILO C182/C29/C105 are near-universally ratified fundamental conventions.
- "cybercrime" -> keep Log: the UN Convention was adopted but, per the retrieved page, not yet in force.

## Keep at Log (soft law or high false-positive risk)
- "ai ethics", "artificial intelligence", "digital compact", "business and human rights", "indigenous peoples": non-binding instruments.
- "right to privacy", "freedom of expression", "hate speech", "social rights": context-heavy; Art. 19/20 sit in tension and a blanket Warn would misfire.
- "corruption", "bribery", "organized crime", "arms trade", "civilians", "non-refoulement": legitimate everyday/news use dominates.

## Existing 7 rules (not modified)
- "exploit" and "abuse" already Block; consider whether those single common words over-trigger (these are very ordinary English words, e.g. software "exploit").
- "child", "minor" RequireConfirmation: very broad words; operator may prefer narrowing (not a change made here).
- "discriminate", "personal data" (Warn) and "self-harm" (RequireConfirmation): no change suggested.

## Live regions (DO, EU) — mapping the pending elevation queue onto real rule conditions (SUGGESTIONS ONLY)

`assets/jurisdiction/elevation-queue/{do,eu}-elevation.json` (status PENDING-OPERATOR-REVIEW) name conditions such as "processing of personal data without lawful basis" or "AI system classified as high-risk under the AI Act". The matcher tests whether the rule's `condition` text (or an alias) appears as a whole word/phrase in the request, so a long descriptive phrase like those will essentially never match a real prompt. The shipped rules use short prompt-natural conditions. If you elevate, elevate THESE (after the 2026-10-28 expansion they exist and are cited):

DO (Dominican Republic) — currently `personal data` = RequireConfirmation (operator-applied), everything else Log:
- `unauthorized access` -> Warn (queue item "unauthorized access to computer systems"; Ley 53-07 art. 6)
- `interception` -> Warn (Ley 53-07 art. 9; Ley 153-98 art. 5 `secrecy of communications`)
- `child pornography` -> Block (Ley 53-07 art. 24) — consistent with the global CRC Art. 34 Block
- `identity theft` -> Warn (Ley 53-07 art. 17; snippet only confirmed the heading, not the penalty)
- `money laundering`, `terrorist financing` -> Warn or RequireConfirmation (Ley 155-17)
- queue item "data breach involving personal data" has no DO rule yet (breach notification was not verifiable from statute text) — add a verified rule before elevating.

EU — currently `personal data` and `data breach` = RequireConfirmation (operator-applied), everything else Log:
- `automated decision` -> Warn (queue item "automated decision-making without human oversight"; GDPR Art. 22)
- `high-risk ai` -> RequireConfirmation (queue item "AI system classified as high-risk"; AI Act Annex III)
- `terrorist content` -> Warn (Regulation 2021/784)
- `child sexual abuse`, `nudification` -> Warn/RequireConfirmation (check the retrieved status in each rule's snippet — some instruments are proposals)
- Do NOT elevate `chatbot`, `cookies`, `copyright`, `sanctions` or `cybercrime`: everyday words with high false-positive rates; keep at Log.

Cost note: every RequireConfirmation match triggers a real model review (fallback walk), and an unjudged review halts the walk as ReviewPending until an operator resolves it. Elevating a common word multiplies both.
