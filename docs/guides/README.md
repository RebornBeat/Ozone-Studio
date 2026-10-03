# Guides index

Real, cross-referenced findings from this session's UI/UX retrospective,
file-beacon design pass, ZCode feature audit, fork/graft comparison, and
coordination/ripple gap analysis — five read-only review forks (R1–R5),
plus this session's own verified fixes, synthesized into one master guide
and topic-specific deep dives. Every claim traces to a real file:line or an
explicit live check; nothing here is inferred or assumed carried-over.

Also published as a formatted page: [Living Graph Field Guide (artifact)](https://claude.ai/artifact/DM6rswrfaZaryceSjb5Gzw)

## Contents

- **[zcode-state-and-remaining-work.md](zcode-state-and-remaining-work.md)**
  — full review of ZCode's state (2026-09-29): the MCP tool ecosystem,
  role-based access control, the modality revival arc, YOLO detection →
  live graph expansion, capture unification — each cross-checked directly
  against source, not taken on the strength of its own notes. Plus one
  deduplicated, prioritized remaining-work list spanning ZCode's backend
  queue, this guide's own UI/UX findings, and the decisions only the
  operator can make.
- **[living-graph-field-guide.md](living-graph-field-guide.md)** — the
  master guide: what shipped this session, plus the full master punch list
  merging every open item below into one ordered to-do.
- **[amt-grafting-vs-claude-code-forks.md](amt-grafting-vs-claude-code-forks.md)**
  — full detail on Ozone-Studio's AMT fork/graft system (the island model,
  `merge_back_to_main`, lineage relations) compared directly against
  Claude Code's own fork mechanism, with real source citations on both
  sides.
- **[ui-ux-retrospective.md](ui-ux-retrospective.md)** — R1's findings:
  the design-token duplication bug (266 hardcoded hex occurrences across
  51 files), stale doc comments, accessibility coverage, and a re-check of
  every previously-flagged-but-not-fixed item.
- **[zcode-feature-extraction.md](zcode-feature-extraction.md)** — R3's
  findings: what's worth adopting from the `~/Projects/ZCode` open-source
  repo (libraries, the subagent directory panel, the theming pattern),
  with an explicit adoption order and an equally explicit list of what NOT
  to adopt.
- **[file-beacon-design.md](file-beacon-design.md)** — R2's findings: the
  real, non-intrusive file-change-notification design (no MetaStrata-style
  wrapping), with the real choke points it reuses and the real gaps
  (no filesystem-watcher crate in the workspace today) it needs to fill.
- **[coordination-ripple-gaps.md](coordination-ripple-gaps.md)** — R5's
  findings: what's already wired into the live graph correctly, and the
  three real remaining silent gaps (task lifecycle, `/config/set`,
  `taskCreate`), plus independent re-verification of this session's own
  MCP jurisdiction+ripple fix.
