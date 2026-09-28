# Creating New MCPs, New Calls, and Absorbing External Software

> Eighth doctrine doc — born from the live builds of 2026-09-27/28 (terminal
> MCP = Pattern A; gamedev absorption = Pattern B; §11's edge-identification
> and the visual MCP = Pattern C trajectory). This is the operational recipe:
> every new capability follows one of these three patterns, and every pattern
> ends at the SAME door — `/mcp/call`, metered, gated, rippled, reviewed.

---

## 0. The two identities every capability has

A capability like "the terminal" is TWO registered things:
1. **The tool** — what it does (`terminal_exec`, `blender_create_object`),
   registered in `/mcp/tools` with version + platform tags.
2. **The MCP** — the secured process that owns execution
   (`tools/terminal-mcp/server.mjs`, the bridges connector layer), which
   the tool entry's endpoint points at.
Neither is "the capability" alone: the tool is the contract, the MCP is the
enforcement point. Ozone sits above both as the reviewer/order layer.

## 1. Pattern A — new MCP for a local capability (the terminal lived pattern)

1. Write the MCP server (any language; Node or Rust): loopback-only HTTP,
   POST `/call` accepting `{tool, agent, input, context}`, answering
   `{success, output}` or `{success, false, error}`.
2. **Security BEFORE capability**: allowlist what it may do (terminal:
   command prefixes, operator rejection, timeout/output caps, optional
   shared-secret token against local bypass). Unconfigured = LOCKED, never
   open.
3. Platform declaration: `platform:<os>/<arch>` capability tag; refuse
   dispatch on mismatch (tag now, native registry field next).
4. Register: POST `/mcp/tools/register` per tool (re-registration
   REPLACES + bumps `server_version` — observed live).
5. Prove it: call through `/mcp/call` — verify the positive path, AND at
   least one refusal path (the terminal's `rm -rf /` and pipe-operator
   refusals are the model).
6. Live as a graph citizen: the call ripples; usage lands in the ledger;
   the insight envelope returns state-of-the-world.

**Persistent-process note (learned the hard way)**: background processes
launched from an agent tool-call session get reaped when that session ends.
Launch long-lived MCPs from the harness's managed background tasks, a
process supervisor, or the operator's own shell — and encode that in each
MCP's README.

## 2. Pattern B — open-source software: absorb, graph, traverse

(gamedev-all-in-one = the lived case)
1. **Download → graph → index → traverse**: place the repo inside Ozone's
   tree as a bounded subpackage (license boundary first — AGPL stays
   AGPL: `tools/bridges/` with its own LICENSE); index its architecture
   into the guide stack; the connectors become the Bridge tier, the tools
   become the Tool tier, redundant surfaces retire.
2. Derive the tool list LIVE (spawn the package's own MCP/CLI as a client,
   call its listing — `listTools`/`list_capabilities`), never hand-copy.
3. One shim connects ALL instances to Ozone: register (ids in the bridge
   range), heartbeat-by-re-registration, `/execute` dispatching to the
   package's own command functions.
4. The 5th-engine rule: new capability = one more entry in the shim's
   table. Zero host changes.
5. Keep everything derivable: if the package adds tools, rerunning the
   registrar updates the registry.

## 3. Pattern C — closed source: reverse engineering, cleanly

The generalization the operator named: a "generalized Wireshark." RE is
not one technique — it is **multi-perspective entry points into the same
unknown**, and each perspective is itself a modality our pipelines already
model:

| Entry point | Capture surface | Ozone home |
|---|---|---|
| **Network wire** | packet capture, protocol re-derivation from observed frames | a Network/RE modality pipeline producing graphs of endpoints, message shapes, state machines |
| **Binary/ABI** | syscall traces, import tables, decompilation diffs | code modality (101) graphs + provisional nodes |
| **Visual/UI** | screen capture → image modality (102): windows, controls, occlusion, affordances (F.3's Spatial*/Affordance types are exactly this) | image modality + a future **visual MCP** |
| **File formats** | structured parse of artifacts the software writes | file/graph pipelines |
| **Behavioral probes** | structured experimentation: invoke, observe delta, record | the terminal MCP + bridge shim pattern (call → observe → hypothesize → re-call) |

**The clean-RE contract** (what keeps it honest and Ozone-shaped):
1. Every hypothesis about the unknown interface becomes a **graph
   candidate** (UnverifiedNode route) — never silent tribal knowledge.
2. Every probe is a **logged call** (capture stores) — experiments are
   data, cited like everything else.
3. Findings graduate to registered tools ONLY with provenance
   (`discovered_via: reverse-engineering`, the observation trail linked).
4. Multi-modality fan-out: the same edge-identification K-registry family
   (§11) runs per perspective in parallel — wire, visual, binary — and
   merges through the graph choke point.
5. Legal/ethical gate: jurisdiction + the tool's own review apply to RE
   calls like all others; RE of software you're not permitted to
   reverse-engineer is refused at the gate, by rule.

## 4. The visual MCP (queued — 6th capability)

Screen/image capture → image modality (102) graphs (windows, controls,
spatial relations, affordances) → exposed as tools (`screen_describe`,
`ui_click`, `ui_read`) through the same contract. This is the
multi-modality RE entry point made operational: the agent SEES software,
not just calls it.

## 5. Versioning + platform rules (learned live)

- Re-registration REPLACES and bumps `server_version` (observed:
  terminal 0.1.0 → 0.2.0). Version is recorded; range-constrained
  dispatch is the designed next step.
- Platform is declared as capability tags (`platform:linux/x64`);
  native field next.
- **The registry is in-memory**: host restarts wipe registrations —
  re-run the registrar(s) after every restart (or the host grows
  persisted registrations; designed, not built).

## 6. The checklist every new MCP must complete (nothing undocumented)

[ ] Security model written BEFORE first execution path
[ ] Locked-by-default when unconfigured
[ ] Refusal path TESTED, not just the happy path
[ ] Platform + version tags on registration
[ ] Registered into /mcp/tools with live-derived list
[ ] Called through /mcp/call only (never direct-to-port from agents)
[ ] Ripple + capture verified (one call, one ledger row, one ripple)
[ ] Persistent-process launch story documented (harness task / supervisor / operator shell)
[ ] Contract container in ZSEI if it introduces new doctrine
