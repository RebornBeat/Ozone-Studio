# Designed-Not-Built Registry — every deferred feature, fully specified (2026-10-29, zcode)

> Operator directive: "capture all … without omission, no build, no tests — just capture all
> edits, ensure all are fully wired in." This registry is the COMPLETE capture of every feature
> that is designed but deliberately not built yet. Each entry is implementation-ready: exact
> touch points, data flow through the guardrails, wiring order, and acceptance rows mapped to
> `MASTER_TEST_GUIDE.md`. Nothing here is a bare name in a queue. When the build gate lifts,
> each entry becomes mechanical work in the listed order.
>
> Status vocabulary: **SPECIFIED** (this document, complete) · **EDITED** (code landed,
> unverified — see the EDITS-ONLY sweep, commit c5dd67e) · **NOT STARTED**.

---

## 1. Screenshot → visual-ingest auto-hook — SPECIFIED

**What**: browser-mcp / browser-farm-mcp screenshots flow into the graph as Image-modality
containers automatically (today the tools only note "feed to visual_ingest" and the operator
does it by hand).

**Why a feature, not a flag**: there is NO host-side route that turns an image file into a
graph container. Faking it with an env var pointing at nothing was refused (no-fabrication
doctrine). The hook needs the route first; then the MCP side is a ~20-line addition.

**Touch points (in wiring order):**
1. **Host route** `src/grpc/mod.rs`: `POST /capture/ingest_image` — body
   `{path: String, project_id: Option<u64>, workspace_id: Option<u64>, session_token: String}`.
   Handler: validate session (already middleware-covered by adding `/capture/ingest_image`
   to the strict gate PROTECTED list — note `/capture/` reads are public today; only the
   ingest route is protected), copy/verify the file exists (screenshots live under
   `{data_dir}/.ozone-context/browser-shots/` and farm-shot dirs — only local paths accepted,
   no URLs), then `runtime.execute_pipeline(102, PipelineInput{ file_path, project_id, … })`
   — the SAME entry the orchestrator's file-graph path uses, so the entry gate, trace root,
   jurisdiction gate, S12 capture, and graph ripple all apply with zero new code. Response:
   `{container_id, success}`.
2. **browser-mcp** `tools/browser-mcp/server.mjs`: after a successful `browser_screenshot`
   write, if `OZONE_BROWSER_INGEST_URL` is set (default OFF), POST
   `{path, project_id: input.project_id ?? null}` to it; the ingest verdict is returned in the
   tool's output as `ingested: {container_id}` or `ingest_error` — a failed ingest NEVER fails
   the screenshot itself (the PNG is already on disk; capture and ingest are separate facts).
3. **browser-farm-mcp** `tools/browser-farm-mcp/server.mjs`: same for `farm_screenshot`,
   env `OZONE_FARM_INGEST_URL` (default OFF). On the EU VPS the URL points at
   `http://127.0.0.1:50051/capture/ingest_image` with the internal token file — same
   out-of-band credential pattern as `OZONE_FARM_API_KEY_FILE`.
4. **Registration note**: ingest POSTs are `agent: "browser-mcp"` / `"browser-farm-mcp"` in
   the usage ledger automatically (route runs under the pipeline's own metering).

**Guardrail flow**: session/strict gate → entry gate (trace root) → jurisdiction gate →
pipeline 102 → container + ripple → S12 row. Nothing bypasses; nothing new to trust.

**Acceptance**: MASTER_TEST_GUIDE **X1** (browser_screenshot → Image container with the PNG,
ripple) becomes mechanically runnable; add **X1a**: ingest failure leaves the screenshot intact
and returns `ingest_error` (honest separation). EU row: farm_screenshot on the VPS lands as a
local Image container on the EU graph.

---

## 2. Farm lens — SPECIFIED

**What**: the Browser-Farm's profiles appear IN the graph-native workspace as first-class
nodes (Command Center + worker profiles, their state, live metrics) — the operator sees the
farm the same way they see every other substrate, not through a separate panel.

**Design decision (recorded)**: farm state enters the UI through the EXISTING `/mcp/call`
door (agent `ui`, paired session, metered, jurisdiction-gated) — NOT a new proxy route.
One exception allowed for read-only posture (below) to avoid per-refresh metering noise.

**Touch points:**
1. **Substrate** `ui/src/graphkit/graphProvider.ts`: after the main load, best-effort fetch
   farm posture: `hostPost("/mcp/call", {tool:"farm_profiles", agent:"ui", input:{}, session_token})`
   and `farm_status`. Parse into a new `GKFarmProfile[]` on the substrate:
   `{profileId, name, mode, status, metrics?}` — null on any failure (honest absence; a
   no-farm deploy shows no farm layer at all). New env of truth: the MCP's `untrusted_*`
   labels stay untrusted — labels are DISPLAY ONLY (X2 rule).
2. **Types** `ui/src/graphkit/types.ts`: `GKFarmProfile`; substrate field `farm: GKFarmProfile[] | null`.
3. **Lens** `ui/src/graphkit/lens.tsx`: `FARM_LENS` — nodeType `"farm-profile"`, accent color,
   layout hint `radial` (Command Center at center, workers around). Registered in the lens
   registry; GraphNativeSection switches to it when `substrate.farm` is non-null and the
   operator picks the Farm tab.
4. **World** `ui/src/graphkit/layout3d.ts`: farm profiles join the world as infra nodes in
   their own cluster (like the modality clusters); edges: profile → command-center
   (`channel` edges, dashed like structural-adjacent), honest labels (status text only).
5. **Acceptance**: new U-row (U15): with the EU farm configured, the workspace Farm lens
   shows the command-center profile RUNNING + workers idle, statuses matching
   `farm_status` verbatim; with no farm, the lens is absent (not an empty shell).

**Not in scope (recorded)**: starting/stopping profiles from the 3D world — those are
`farm_profile_start/stop` actions that belong to the acting chain (a proposal or a direct
operator tool call), not a lens click. The lens is a VIEW.

---

## 3. 3D call-graph lens — SPECIFIED

**What**: code containers render their inner call graph in the 3D world — functions as
nodes, Calls/Extends/Implements as edges, laid out by call depth — instead of the current
single container node.

**Known honest boundary** (CC's finding, kept visible): Calls edges are SAME-FILE only
(regex on the code pipeline); cross-file resolution doesn't exist (T-C4 asserts the boundary).
The lens renders what's stored and labels the boundary in-world (EdgeRealismLegend already
carries the class system).

**Touch points:**
1. **Data**: the provider fetches the code container's CONTENT (it already holds container
   ids; content via the existing GetContainerContent path used by graphViewData) — the
   persisted `PersistedCodeGraph` JSON carries files → functions + typed edges (Calls,
   Extends, Implements, same-file). New substrate index: `callGraphByContainer:
   Record<number, {nodes: {id,name,kind}[], edges: {src,dst,class}[]}>`, populated lazily on
   first focus of a code-graph hub (fetch once, cache).
2. **Layout** `ui/src/graphkit/layout3d.ts`: `layoutCallGraph(containerId)` — assign each
   function node a call-DEPTH layer (longest path from any root over Calls edges, cycles
   broken by first-visit order), position by layer (Y) + in-layer arc (X/Z); Extends/
   Implements render as their own edge classes (dash styles exist). Cross-file boundary:
   functions in the same container only; a Calls edge naming a missing target renders as a
   tagged stub node "external: name" (real: the edge exists, the target doesn't — honest).
3. **Lens** `lens.tsx`: the Code lens gains `layout: "calls" | "cluster"` (operator toggle
   in the inspector; default stays cluster).
4. **Legend**: EdgeRealismLegend gains the "external stub" entry.
5. **Acceptance**: T-C4 runs VISUALLY: one fixture file with same-file call + extends +
   implements → 3 edge classes render; a deliberate cross-file call → the tagged external
   stub; T-C5 fixtures per language unchanged.

---

## 4. In-world indicators — SPECIFIED

**What**: the GraphKit indicators (currently in the header rail / inspector) ALSO appear
IN the 3D world attached to the objects they describe — plus in-world controls.

**Design (maps 1:1 to the existing indicator set — no new data sources):**
| In-world object | Indicator | Data source (existing) |
|---|---|---|
| Task container nodes | TaskStateIcon as a billboard sprite on the node | substrate task refs (already loaded) |
| Gate-reviewed stages/plans | GateState chip on the plan/AMT hub | stage summary already parsed for OrchestrationStatusPanel |
| File nodes | tiny sourceFile badge (reverse weave, EDITED) | substrate.sourceFile |
| Farm profiles (lens §2) | status dot (running/paused/idle) | farm posture fetch |
| Capture stores | one world-corner "capture pillar" with S10–S13 bars | the same /capture/* totals CaptureChip polls |
| Loop health | hub ring pulse when amt/meta/i/assistant heartbeat present | LoopChips data |

**Controls (minimal, honest):** click = select (exists); double-click = focus (exists);
right-click = in-world menu: Pin label / Hide subtree / Copy container id. NO teleport,
NO "execute from world" — actions belong to the acting chain (§2 note applies).

**Touch points:** `ui/src/graphkit/GraphWorld3D.tsx` (sprite layer + raycast already
exists for picking; add a per-node optional `indicator` field fed by the substrate),
`layout3d.ts` (attach points), `indicators.tsx` (sprite variants of TaskStateIcon/GateState
— same glyphs, rendered to canvas textures). All values real; absent data = no sprite.

**Acceptance**: U-row: task nodes in a project with a running task show the running glyph;
a project with none shows none; capture pillar matches CaptureChip numbers exactly.

---

## 5. Vault — SPECIFIED (the Ozone Vault; replaces Browser-Farm's plaintext vault pattern)

**Doctrine (fixed, from the FULL-STATE capture):** secrets never in graph containers, never
in prompts/LLM context, never in logs/ledger rows; access via short-lived broker leases;
plaintext reveal is human-only and audit-noisy.

**Design:**
- **Storage**: `{data_dir}/vault/` — a single encrypted file (`vault.bin`, XChaCha20-Poly1305
  via `chacha20poly1305` crate) + Argon2id key derivation from the OPERATOR's master
  passphrase (never stored; unlock is interactive/CLI only). Items typed:
  `ApiKey | Credential | Token | Note`, each `{id, kind, label, secret, meta, created_at,
  rotated_at}`. No graph container is created for any item — the vault's existence may
  ripple (an event "vault.rotated label=X"), never its contents.
- **API surface** (`src/vault.rs` new module + routes in grpc/mod.rs, ALL strict-session,
  ALL operator-role):
  - `POST /vault/unlock` {passphrase} → session-scoped unlock (in-memory, expires with session)
  - `POST /vault/set` {kind,label,secret,meta} → id (secret encrypted at rest immediately)
  - `POST /vault/lease` {label, ttl_secs ≤ 300, purpose} → `{lease_token, expires_at}` —
    the ONLY way tooling touches a secret
  - `POST /vault/reveal` {label} → REFUSED unless the request carries a physical-presence
    proof: the operator must re-enter the master passphrase (interactive path only — CLI
    `ozone-studio vault reveal <label>`); the HTTP route intentionally has no reveal
  - Every set/lease/reveal/unlock attempt (success or fail) → append-only
    `{data_dir}/vault/audit.jsonl` (ts, action, label, session_id, outcome) — no secret bytes.
- **MCP broker** (phase 2): `tools/vault-mcp/server.mjs` — tools `vault_status`,
  `vault_lease` (returns the lease TOKEN, not the secret; the consuming tool server
  exchanges the lease token with the host at its own /call time). No `vault_get` tool exists.
  Registered like any MCP family; metered like any MCP family.
- **Integration**: Browser-Farm's plaintext account vault is the first consumer story —
  farm profiles pull credentials via lease at profile-start time on the VPS (farm-side change,
  phase 3). OpenRouter key in config.toml moves to the vault (operator-gated rotation day).
- **Build order**: chacha20poly1305 + argon2 crates (deps edit) → src/vault.rs + routes
  (build) → CLI subcommand (build) → tests (roundtrip, wrong-passphrase, lease expiry,
  audit-no-secret-bytes, reveal-refused-over-HTTP) → MCP broker → farm integration.
- **Acceptance**: S-row additions: grep the whole {data_dir} for a known secret after set →
  only ciphertext bytes; lease expired → consumer refused; audit.jsonl has no secret bytes ever.

---

## 6. PDF/DOCX pipeline — SPECIFIED

**What**: `.pdf` / `.doc` / `.docx` (canonical modality: **Text**) currently reach text
pipeline 100, which has ZERO pdf/docx awareness — a binary PDF fed to a plain-text reader
is garbage-in. Law-book ingestion (the operator's stated goal) is blocked on this.

**Decision (recorded)**: extend the EXISTING text pipeline (id 100) — a separate
"Document" modality would fork the text graph machinery for no gain; PDFs ARE text
documents once extracted.

**Touch points:**
1. **`assets/pipelines/modalities/text/`**: add `extract/` module —
   `pdf.rs` (the `pdf-extract` crate — pure-Rust, no poppler system dep) and
   `docx.rs` (`docx-rs` reader or unzip+`quick-xml` over `word/document.xml` — the
   docx format is a zip of XML; unzip via the `zip` crate). Dispatch in main.rs by
   extension BEFORE the plain-text read: pdf → pdf-extract → plain text; docx → XML
   extraction → plain text; everything else → current path unchanged. Extraction failure →
   honest structured error (never silent empty-graph success — the confetti doctrine
   applies to extraction too).
2. **Page provenance**: pdf-extract yields per-page text — chunk metadata carries
   `page: N` so citations can cite pages (a real law-book requirement: "see p. 214").
3. **Registration**: NONE needed — same pipeline id, same modality; `detect_file_modality`
   (EDITED, unified) already routes pdf/doc/docx → text.
4. **Tests** (pipeline-local, run at the build gate): one single-page PDF, one multi-page
   (page numbers in chunk meta), one scanned/image-only PDF (honest "no text layer" error —
   OCR is explicitly OUT of scope, recorded), one .docx with headings (headings become
   section boundaries), one encrypted PDF (honest error).
5. **Acceptance**: MASTER_TEST_GUIDE L8 Text-group rows gain: a real PDF attaches → Text
   graph containers with page-carrying chunks; the law-book plan unblocks.

---

## 7. Capture-completeness matrix — every deferred item, where it is captured

| Item | Registry entry above | Test rows | Other captures |
|---|---|---|---|
| Screenshot→ingest hook | §1 | X1 (+X1a new) | CHECKLIST edits-sweep |
| Farm lens | §2 | U15 (new) | BROWSER_AUTOMATION_GUIDE "Open" list |
| 3D call-graph lens | §3 | T-C4/T-C5 visual | MASTER_REMAINING_WORK_GUIDE (call-graph findings) |
| In-world indicators | §4 | U-row (new) | MASTER_REMAINING_WORK_GUIDE W5; ACTING_FACTORS_REGISTRY |
| Vault | §5 | S-rows (new) | MASTER_REMAINING_WORK_GUIDE (vault design + plaintext findings) |
| PDF pipeline | §6 | L8 Text rows (new) | MASTER_REMAINING_WORK_GUIDE (modality defects); law-book unblock |
| OpenRouter key rotation | n/a — operator action | S-row exists | CHECKLIST, FULL-STATE capture |
| Elevation queue | n/a — operator review | jurisdiction rows | assets/jurisdiction/elevation-queue/ + ELEVATION_CANDIDATES.md |
| Data mutations (FileRef backfill, dedupe, worktrees) | n/a — operator-gated ops | — | MASTER_REMAINING_WORK_GUIDE; earlier CHECKLIST entries |
| DOMorpher compile campaign | **EDITED** (c5dd67e: 29 lexer fixes + Cargo.toml) | DOM rows in L1 | CHECKLIST edits-sweep |
| koi S11 analysis / valence digest | n/a — acceptance checks | A-rows | memory + earlier CHECKLIST |
| CC jurisdiction continuation | n/a — CC's live assignment | jurisdiction rows | shared-context assignment note |

**Nothing else is deferred-and-uncaptured.** The EDITS-ONLY sweep items (modality
unification, CapturePresence, reverse file weave, launcher) are LANDED UNVERIFIED and
await the build gate with the rest.
