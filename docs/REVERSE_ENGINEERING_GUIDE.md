# Reverse-Engineering MCPs — the generalized-Wireshark doctrine for absorbing unknown software

> Ninth doctrine doc; expands NEW_MCP_GUIDE.md Pattern C into the full
> standalone guide. Operator framing: **"clean multi cross — bigger than
> Wireshark and cleaner."** Wireshark observes ONE surface (the wire). This
> doctrine observes EVERY surface a software artifact exposes — wire,
> binary, visual, storage, behavior — through modality pipelines, and
> lands every finding as registered MCP tools and modality graphs. It is
> multi-tool and multi-modality by construction, and it extends to mobile
> applications (Android/iOS) as first-class targets.

---

## 1. The principle

Unknown software is an unknown GRAPH. Reverse engineering is the
disciplined construction of that graph from observations, where every
observation is a logged probe, every hypothesis is a graph candidate, and
every stable finding graduates into a registered capability — a tool in
`/mcp/tools`, backed by a connector, invoked through `/mcp/call` under the
same security posture as everything else.

Nothing is "reverse-engineered" in a human's head and hand-written into a
one-off script. The system learns the software the way it learns
everything: **structure before intelligence**.

## 2. The observation surfaces (each one is a modality)

| Surface | Captured by | Becomes | Ozone home |
|---|---|---|---|
| **Wire** (network traffic) | packet capture, TLS-aware proxies, MITM with operator-installed certs | endpoint graphs, message schemas, state machines, timing | network modality (123) + RE graphs |
| **Binary/ABI** | strace/ltrace, import tables, decompilation | call graphs, dependency graphs, API surfaces | code modality (101) |
| **Visual/UI** | screen capture → image 102; UI automation trees | window/control graphs, spatial + affordance relations (F.3 types) | image modality (102) + visual MCP |
| **Storage** | files/databases the software writes | schema graphs, format definitions | file/graph pipelines |
| **Behavior** | structured probes: invoke → observe delta | behavioral edges (Causes, Enables, Requires) | behavioral graphs via RE loop |

Multi-modality is the power: the SAME unknown software is observed from N
surfaces in parallel, and the graphs MERGE — a UI button (visual graph)
that triggers an HTTP call (wire graph) that writes a config file
(storage graph) is ONE behavior, reconstructed from three views. That
merge is the edge-identification family's cross-modality fan-out, applied
to RE.

## 3. The mobile extension (phone apps as first-class targets)

Android:
- **APK unpacking**: apktool/jadx → resources, manifest, smali → code
  modality graphs (the declared surface: activities, services,
  permissions, exported components).
- **Wire**: mitmproxy with a user-installed CA on the device/app (pinning
  handled per-app: objection/frida for authorized testing).
- **On-device observation**: frida-server instrumentation — function-level
  hooks whose traces feed the behavioral graph.
- **Graduation**: discovered endpoints/commands → connector module →
  registered tools (`<app>_<operation>`) — the phone becomes a BRIDGE
  (same contract as Roblox/Unity: register, heartbeat, execute).

iOS: same shape — ipa unpacking (class-dump), frida instrumentation,
network capture via a proxy the phone trusts. Graduation identical:
phone = bridge, operations = tools.

**The mobile end-state the operator named**: the phone app is soon
itself an Ozone surface — the app connects as a bridge (multi-tool
modality), its UI is observable through the visual modality, and its
capabilities register as tools.

## 4. The RE loop (per target, in order)

1. **Declare the target** — a real container (RETarget) with scope,
   legality note, and operator approval (§6).
2. **Observe** — spin up capture per surface (wire proxy, strace, screen
   recorder, storage watcher). Every capture lands in the relevant
   modality pipeline. Probes are logged calls, never interactive folklore.
3. **Hypothesize** — from merged graphs, propose interface candidates:
   "this binary exposes operation X over endpoint Y with schema Z." Each
   hypothesis = an UnverifiedNode graph candidate (the existing AMT route).
4. **Probe** — structured experiments through the terminal/bridge tools:
   invoke, observe delta, confirm or refute. Every probe captured
   (S10–S13 family). Confidence comes from verification, never guessing.
5. **Graduate** — a confirmed operation becomes a connector function +
   registered tool with provenance `discovered_via:
   reverse-engineering`, citations linking the probe trail. Unverified
   operations stay candidates, visible and labeled.

## 5. What "cleaner than Wireshark" means concretely

- **Wireshark shows bytes; this builds tools.** The output of an RE pass
  is not a capture file — it's a registered capability the whole system
  can call under the standard review/metering posture.
- **Multi-surface by default** — wire AND visual AND storage AND behavior,
  fanned out in parallel per modality, merged at the graph.
- **Graph-native memory** — everything observed is in the living graph,
  traversable, rippling, citable. A re-observation updates; nothing is
  re-learned from scratch.
- **Guardrailed** — jurisdiction runs on RE probes like every call; the
  legality/scope declaration is a container field, not a footnote;
  operator approval is explicit per target.

## 6. The guardrails (operator: "I am putting guardrails on mine")

- Per-target legality container: scope + jurisdiction basis + approval
  reference, required before the first probe.
- RE probes never run on targets outside the declared scope — the gate
  checks the call against the target container.
- Copyright/DMCA/anti-circumvention surfaces flagged at the gate
  (interoperability and security-research purposes recorded per target).
- China/EU/US-state-specific rules respected through the existing
  jurisdiction rule sets — the operator loads the rule sets; the gate
  enforces them on every RE call.

## 7. Build order (queued)

1. RETarget container type + approval field (small, additive)
2. Wire-surface collector: mitmproxy sidecar → flow summaries → network
   graph candidates
3. Visual-surface loop: visual MCP captures → image 102 → UI graphs
   (operational today for X11; portal/Electron for Wayland)
4. Probe runner: terminal/bridge-driven structured experiments with S13
   capture
5. Graduation path: confirmed findings → connector + tool registration
   with provenance
6. Mobile connector template (Android first: frida + mitmproxy +
   apktool), then iOS
