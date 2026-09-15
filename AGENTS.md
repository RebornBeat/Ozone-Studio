# Ozone-Studio — workspace agent instructions

## Shared context protocol (ZCode + Claude Code + Ozone-Studio)

This repo runs a 3-way agent coordination tool. Before starting any work:

1. Call MCP tool `context_summary` (server: `ozone-shared-context`) — who is
   live, which files are claimed, latest handoffs.
2. `file_claim` any file before editing; `file_release` when done. Refusal =
   another live agent holds it — coordinate, don't overwrite.
3. End sessions with a `handoff` note (`note_add`).

Full protocol: `tools/ozone-shared-context/README.md`. State lives in
`.ozone-context/` (git-visible). Events mirror to the Ozone-Studio host
monitor (`:50051`).

## House rules

- Parallel agents may be active (Claude Code runs concurrently) — never
  edit a file another agent holds a claim on.
- `CHECKLIST.md` is Claude Code's living status record — read it, update
  additively, never rewrite.
- No fabricated metrics anywhere: token counts, presence, claims are real
  captured values or explicitly absent.
