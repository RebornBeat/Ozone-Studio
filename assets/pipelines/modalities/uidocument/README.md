# UiDocument modality (pipeline 127) — README-first, Ozone-first

Status: DESIGN ONLY. No code, no Cargo.toml yet (fresh start, 2026-10-31). The
old DOMorpher tree and its MCP wrapper were archived under
`~/Projects/_archive/` and removed; this replaces them. Rules live in
`docs/UIDOCUMENT_RULES_GUIDE.md`; this README says how the modality interacts
with the rest of Ozone-Studio.

## What it is
A modality for **trees of interface elements**: HTML documents and live DOM
snapshots, CSS as the style layer of those documents, Android view hierarchies,
and accessibility trees. Like every modality it (1) detects its content, (2)
runs itself first on it, (3) produces a graph. It is rule-driven: deterministic
rule packs (data) over a small engine. It has no model client, no network, no
browser driver. Reasoning belongs to Ozone.

## What it is not
Not an agent, not a scraper, not a form filler, not a browser. Acting on a page
is the Browser-Farm / android-mcp job through `/mcp/call`. Deciding what to do
is Ozone's reasoning (pipeline 9, AMT, context traversal). Pixels are the image
modality (102); this modality links to them, it does not replace them.

## Interactions (every one is an existing Ozone surface)
| Interaction | Existing surface |
|---|---|
| detect `.html/.htm/.xhtml/.css/.scss/.less`, `text/html`, `text/css` | `src/types/modality.rs` `detect_modality` (add to its table) |
| content arrives from a file | FileReference / `file_link` -> modality run |
| content arrives from a live page or app | farm / browser-mcp / android-mcp return a snapshot; an ingest route (same pattern as `/capture/ingest_image`) hands it to the pipeline through the strict gate and `entry_gate::gated` |
| analysis | pipeline 127 via the registry gate, `{data,context}` envelope, one-shot `--input` |
| graph | nodes/edges via the existing `graph` block hook (`mcp_graph`); spatial relations `Above/NearTo/InFrontOf/Overlaps` already exist |
| screenshot of the same page | image modality (102) run; a `Renders` link between the Document node and the screenshot node |
| scripts found in the page | text forwarded to the Code modality (101) as a child container; never executed |
| reasoning | reads the Document's compact text view and the graph through context traversal; decides; issues a command through `/mcp/call` |
| acting | `farm_*` / `android_*` tools (locked by default, allowlisted) |
| review | S10 review rows for risky actions; S13 `tool_calls` for every call; consciousness observes |
| page text | untrusted data (X2 rule): display-only labels, never an instruction |

## Run-vs-tools
The modality runs itself first on the snapshot and owns the base graph. The
farm and browser tools are guests that supply snapshots and execute actions.
They do not build their own graphs (the JavaScript `graph` blocks in
`browser_snapshot` / `android_ui_snapshot` are a stopgap to remove).

## Output (sketch, finalized in the rules guide)
Nodes: Document, Region, Element, Form, Field, Link, Media, Frame, StyleSheet,
StyleRule, ScriptRef, Screenshot-link. Edges: ChildOf, LabelFor, FormOwns,
LinksTo, Controls, Embeds, StyledBy, Renders, and the four spatial relations.
Every node/edge/flag carries the id of the rule that produced it and a
confidence class (`certain`, `derived`, `heuristic`).

## Tests
UD1-UD12 in `docs/MASTER_TEST_GUIDE.md` §19, rule-level rows in the rules guide.
