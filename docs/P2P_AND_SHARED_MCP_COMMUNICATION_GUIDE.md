# P2P and Shared-MCP Communication — Three Subsystems, Disambiguated

> **STATUS CORRECTION 2026-10-23 (read before acting on this document):**
> parts of this guide are stale relative to the tree. Verified current facts:
> the I-Loop EXISTS and runs (`src/orchestrator/i_loop.rs`, boot-spawned,
> live on the local + EU hosts); the ripple actors EXIST
> (`src/orchestrator/actors.rs` — SecurityFinding / CoordinationFinding /
> TaskFailed / CodeStaleness, boot-registered) and a sensor tick now feeds
> them; the findings dedupe key is FIXED (finding-routes dedupe on
> container+route+detail-digest — global findings no longer drop each other);
> the P2P trust gate is BUILT and BIDIRECTIONAL in the tree (paired_peers.json,
> default-deny both directions) with a peer-sync review pass surfacing
> received signals as finding containers; the queue processor exists behind
> OZONE_AUTO_EXECUTE_TASKS=1 (approval-gated). Authoritative status:
> `CHECKLIST.md` (2026-10-23 entries) + `docs/STATE_CAPTURE.md`. This
> document's prose below is retained as design history, not current state.

> Written to answer one operator question directly: given paired devices
> and paired Ozone-Studio instances across jurisdictions (e.g. a local
> machine and a Contabo EU VPS), and given that Claude Code and ZCode
> *already* coordinate on this repo through a shared MCP server
> (`tools/ozone-shared-context/`), wouldn't that same shared-MCP approach
> be the best fit for communication between two separate Ozone-Studio
> host instances — or even just between AI coding agents?
>
> Short answer, argued in full below: **no, not for host-to-host.** The
> shared-MCP pattern, as it exists today, is a *local, one-filesystem*
> mechanism — it is already the right tool for what it does (agents
> coordinating on one repo clone), but it is not a network channel
> between two physically separate hosts, and stretching it into one would
> mean re-deriving transport, encryption, and auth that the native P2P
> layer already has built in. The native P2P layer is the correct
> mechanism for host-to-host, cross-jurisdiction sync — it has real,
> currently-open bugs, but they are bugs in an otherwise correctly-aimed
> design, not a structural ceiling the way "one shared filesystem" is for
> MCP.
>
> Everything below was verified against the live code during this pass
> (file/line references are informal pointers, not guarantees the line
> numbers are stable) — nothing here is carried over from an earlier
> session's notes without being re-checked.

---

## 0. The three subsystems — do not conflate these

This repo has three genuinely different mechanisms that all involve the
word "connect" or "pair," and this session found real confusion risk
between them. Keep the terminology below fixed for the rest of this
document.

| | **Device pairing** | **Native P2P sync** | **Shared MCP context** |
|---|---|---|---|
| **What it connects** | The operator's own devices (phone, laptop, web client) to **one** Ozone-Studio host | Two or more **separate, independently-deployed** Ozone-Studio host instances | Two or more **AI agents** (Claude Code, ZCode, …) working on **one shared repo clone** |
| **Files** | `src/pairing.rs`, `src/auth/mod.rs` | `src/network/mod.rs` | `tools/ozone-shared-context/server.js` + `.ozone-context/` |
| **Routes / entry points** | `POST /pairing/start`, `GET /pairing/status`, `POST /pairing/approve`, `POST /auth/challenge`, `POST /auth/authenticate` | `[network]` in `config.toml`, `POST /network/connect-peer` | MCP tool calls (`context_summary`, `file_claim`, `note_add`, …) over stdio |
| **Transport** | HTTP + Ed25519 challenge-response, or QR-code possession | TCP + noise (encryption) + yamux (multiplexing), via libp2p | **stdio** — a local child process per agent, no network listener at all |
| **Crosses a network/jurisdiction boundary?** | No — both ends talk to the *same* host | Yes — this is its entire purpose | **No, not today** — see §3 |
| **State model** | Session tokens in `AuthSystem`'s existing session store | gossipsub pub/sub of domain containers (methodologies, blueprints, findings) | Append-only notes/claims/handoffs, flat JSON files on disk |
| **Scope** | One host's own client devices | Cross-jurisdiction instance-to-instance | One repo clone / one shared filesystem |
| **Current real status** | Fully working | Domain logic (what should sync and when) is real and wired; the libp2p transport itself has two confirmed open bugs — see §2.3 | Fully working, in live use this very session |

The rest of this guide expands each column, then answers the operator's
actual question in §4.

---

## 1. Device pairing — one host, the operator's own devices

`src/pairing.rs` opens with its own doc comment, which is accurate and
worth quoting directly rather than re-describing:

> "QR device pairing — the phone as authenticator.
> Ozone-Studio is multi-device: the same host serves the desktop UI, a
> web UI, and (eventually) the mobile app. New devices join securely
> without passwords or typed secrets:
> 1. The device asks the host to start a pairing session and shows the
>    returned QR code (plus a human-readable code).
> 2. The user's phone — already trusted, already in hand — scans it. The
>    QR encodes a short-lived approve URL on the host itself.
> 3. The phone approves with one tap. The host creates a REAL session
>    through the existing `AuthSystem` (same session store, same expiry
>    policy as Ed25519 logins) and attaches a registered device.
> 4. The pairing device, polling `/pairing/status`, receives the session
>    token and is in — multi-device, no key ever typed or displayed.
> Possession of the QR is the factor: the code is 8 unambiguous chars,
> the session expires in minutes, and approval requires reaching the
> host."

Concretely: `PairingHub::start` creates a `PairingSession` (random
`pairing_id`, an 8-character code from an ambiguity-free alphabet,
`PAIRING_TTL_SECS = 300`). `PairingHub::approve` consumes the code and
calls `AuthSystem::create_pairing_session`, which mints a real `Session`
through the *same* session store used by the raw Ed25519
challenge-response path (`POST /auth/challenge` → `POST
/auth/authenticate`, `src/auth/mod.rs`, Ed25519 signatures, no
passwords). The routes are wired in `src/grpc/mod.rs`:
`/pairing/start`, `/pairing/status`, `/pairing/approve`, `/auth/challenge`,
`/auth/authenticate`.

**This is "multiple clients, one host" — not inter-host communication at
all.** A phone that pairs to a host does not become a peer of that host
in any sense the P2P layer would recognize; it becomes a device with a
session token good against that one host's own `AuthSystem`.

**Note for a future guide (the operator's own framing, captured here
rather than built out — this is explicitly a "note for later," not the
focus of this document):** when a "paired device" is the operator's own
device — a phone or laptop, not a separate autonomous Ozone-Studio
instance — what it actually gets is session/state access to that single
host: it reads and drives that host's existing data and conversation
state, nothing more. That is a materially simpler case than everything
else in this guide, worth remembering as its own category if a more
detailed pairing guide is ever written, and it should not be confused
with instance-to-instance sync just because both involve the word
"pairing."

---

## 2. Native P2P instance-to-instance sync

### 2.1 What it is

`src/network/mod.rs` is a real libp2p-based transport: `gossipsub` (pub/sub
messaging), `mdns` (local peer discovery), `identify`, and `ping`,
combined into one `OzoneBehaviour` built with noise-protocol encryption
and yamux multiplexing (`SwarmBuilder::with_tcp(tcp::Config::default(),
noise::Config::new, yamux::Config::default)`). Configuration lives under
`[network]` in `config.toml`:

```
[network]
enable_p2p = true
enable_cloud_sync = false
p2p_port = 9090
max_peers = 50
enable_mdns = true
batch_sync_interval_secs = 60
```

`POST /network/connect-peer` (`src/grpc/mod.rs`) is the operator-triggered
entry point to dial a new peer by multiaddr
(e.g. `/ip4/161.97.114.105/tcp/9090/p2p/<PeerId>`) — this is exactly the
shape of connection that would join, say, this session's real local-DR
machine to a Contabo EU VPS instance.

### 2.2 Sync priority — quoted verbatim from the module's own doc comment

`src/network/mod.rs`'s header comment states the real, current sync
rules exactly:

> "SYNC BEHAVIOR:
> - Methodologies: ALWAYS sync (no significance check)
> - Blueprints: ALWAYS sync (no significance check)
> - New Container Findings: ALWAYS sync (queue for batch)
> - Pipeline Results: ON-DEMAND only (explicit share request)
> - User Data: NEVER sync (local only)"

This maps onto the real `SyncPriority` enum (`High` / `Normal` / `Low` /
`Never`) and the real `on_methodology_created` / `on_blueprint_created` /
`on_container_created` / `on_pipeline_completed` methods — not aspirational
naming, these are the actual call sites. High-priority items trigger an
immediate `broadcast_high_priority()`; normal-priority items queue and
flush on `process_batch_sync()` (driven by `batch_sync_interval_secs`);
low-priority (pipeline results) only sync when a caller explicitly passes
`share_requested = true`; `Never` items are simply never queued.

The domain-level wiring from live graph activity into this sync queue is
real and already connected: `orchestrator::amt_loop::spawn_network_hook_bridge`
subscribes to the `GraphEventHub` and, on every `"created"` event, calls
the matching `on_methodology_created` / `on_blueprint_created` /
`on_container_created` method based on the container's real type — gated
by `NetworkConfig::auto_broadcast_sync` at its one call site in
`src/lib.rs`. So the *domain logic* ("what should sync, and when") is not
the broken part of this system.

### 2.3 Current real state of the transport — verified live, this pass

Two gaps were found this session at the libp2p transport layer itself —
below everything in §2.2, which is correctly wired but can only matter
once the transport actually connects peers.

**Bug 1 — `swarm.listen_on()` was never called — fixed, present in the
working tree, not yet committed.** `NetworkManager::initialize()` now
contains a block (confirmed by reading the current file and its
`git diff` against `HEAD`) that calls `swarm.listen_on(...)` on
`/ip4/127.0.0.1/tcp/{p2p_port}`, with its own comment dating the fix to
2026-10-20 and explaining the root cause directly: before this, neither
this host's nor a paired EU VPS instance's real listening socket (checked
with `ss -tlnp`) had anything bound on `p2p_port` despite both configs
setting it to 9090 — so every `connect_peer` dial from either side was
always going to fail with connection-refused, regardless of firewall
state or address correctness, and `peer_count` could never become
nonzero. The bind is loopback-only by design, matching this codebase's
existing convention for the gRPC/HTTP server (`[grpc] address =
"127.0.0.1"`); real cross-host reachability is meant to go through an SSH
tunnel to this port, the same pattern already used to reach the HTTP API.
This change is present in `git status`/`git diff` right now as an
uncommitted modification to `src/network/mod.rs` — it exists in the
working tree at the time this guide was written, not merely proposed.

**Bug 2 — the libp2p `Swarm`'s own event loop is never polled anywhere
in the codebase — confirmed still open, this pass.** A direct search of
the entire codebase (`grep -rn "select_next_some\|SwarmEvent\|poll_next"
src/ assets/`) returns **zero matches**, anywhere. This matters because
libp2p's `Swarm` is a `Stream`: dialing a peer only *starts* the TCP
connect; the noise handshake, yamux setup, `identify` exchange, and
gossipsub mesh formation that make a connection real all happen as
`SwarmEvent`s that must be pulled out of the swarm by polling it (typically
a `loop { swarm.select_next_some().await }` task). Nothing in this
codebase does that — not in `src/network/mod.rs`, not in `src/lib.rs`
(where `NetworkManager::new`/`initialize` are called), not in
`orchestrator/amt_loop.rs`'s `spawn_network_hook_bridge` (which only
calls the already-existing `on_*_created` hook methods, never touches the
swarm directly). The practical consequence: even after Bug 1's fix, a
`connect_peer` dial can open a raw TCP socket, but libp2p's own
connection-establishment state machine never advances, so
`swarm.is_connected(peer_id)` and `swarm.connected_peers()` — which
`send_to_peer`, `broadcast`, and `get_status` all depend on — will not
reflect real connectivity. `HealthResponse.peer_count` (`src/grpc/mod.rs`)
is honestly wired to the real `NetworkManager::get_status()` value (its
own comment notes it "was previously only faked in the UI layer via a
nonexistent config field," and that UI-layer fake has already been
removed) — but an honest read of a number that cannot yet become nonzero
is still an honest zero, not a working peer connection. **As of this
pass, this is the one remaining blocker between "dials correctly" and
"two instances actually sync data."** It is a real, currently-open gap —
not fixed in this pass, and this guide makes no claim about whether work
on it is happening concurrently elsewhere in this session; check
`git log`/`git diff` again before relying on this section staying
accurate.

### 2.4 Native properties of this layer (true regardless of the two bugs above)

- **Built into the product binary** — compiled into the Rust binary
  itself. Any deployed Ozone-Studio instance has this capability with
  zero extra process, port, or dependency to stand up beyond what the
  binary already is.
- **Transport-level encryption and multiplexing already part of the
  design** — noise protocol (encryption) and yamux (stream multiplexing)
  are in the `SwarmBuilder` chain today, not a planned addition.
- **Designed explicitly for crossing real network/jurisdiction
  boundaries** — it dials real multiaddrs over TCP; it is not
  filesystem-bound in any way.
- **Pub/sub fan-out via gossipsub** — a single `broadcast()`/publish call
  reaches every connected peer in a topic; the sender never has to
  enumerate peers one at a time, which suits "a new methodology should
  reach an arbitrary number of other instances" far better than a
  point-to-point design would.
- **Local discovery for free** — `mdns` finds peers on the same LAN
  without any configuration, useful when instances are not
  cross-jurisdiction at all but just multiple machines on one network.

---

## 3. Shared MCP coordination context — what CC and ZCode use today

### 3.1 What it actually is, verified against the real server code

`tools/ozone-shared-context/server.js` is, by its own README's words (`tools/ozone-shared-context/README.md`):

> "ZCode + Claude Code + Ozone-Studio share one coordination state. The
> canonical store is `.ozone-context/` in this repo (git-visible); events
> mirror live into Ozone-Studio's monitor feed (`:50051`). The MCP server
> is zero-dependency Node: `tools/ozone-shared-context/server.js`."

Reading `server.js` directly confirms the transport, rather than taking
the README's word for it:

- `process.stdin.setEncoding("utf8"); process.stdin.on("data", (chunk) => { ... })` —
  the server reads JSON-RPC requests off **stdin** and replies with
  `process.stdout.write(JSON.stringify({ jsonrpc: "2.0", id, result }) + "\n")`
  on **stdout**. There is no `http.createServer`, no `.listen()` call —
  no network listener of any kind in this file.
- Its own self-registration call (which POSTs to the host's
  `/mcp/tools/register` purely so the tool shows up in `GET /mcp/tools`
  and the Monitor tab) announces itself honestly:
  `transport: "stdio"`.
- `STATE_DIR` defaults to `process.env.OZONE_CONTEXT_DIR ||
  path.join(process.cwd(), ".ozone-context")` — a **local filesystem
  directory, resolved relative to the process's own current working
  directory** unless overridden.
- `.mcp.json` in this repo launches it as: `{"command": "node", "args":
  ["/home/rebornbeat/Projects/Ozone-Studio/tools/ozone-shared-context/server.js"],
  "env": {"OZONE_HOST": "http://127.0.0.1:50051", "OZONE_AGENT_NAME":
  "claude-code"}}` — i.e. each agent's MCP client (Claude Code via
  `.mcp.json`, ZCode via `.zcode/config.json`, per the README's own
  wiring table) spawns **its own local child process** of this same
  script, pointed at the same local `.ozone-context/` directory.

**So, stated plainly: this mechanism, as it exists today, is a local
process (stdio, no network listener) reading and writing a local
filesystem directory.** It is scoped to "multiple AI agents working on
one shared repo clone on one machine (or a filesystem all parties can
see)" — not currently a live network channel between two physically
separate Ozone-Studio deployments in two different jurisdictions. The
one network call this server makes (`fetch` to
`/mcp/tools/register`/the host's monitor) is a one-way, best-effort,
fire-and-forget announcement for discovery/display purposes
(`.catch(() => {})` swallows any failure) — it is not the data channel
notes/claims travel over, and it does not make the coordination state
itself reachable over the network.

### 3.2 What the real protocol gives today (and why it works well for its actual job)

Per the README and this session's own live use: `context_summary` (one
call: live agents, claimed files, latest handoffs/findings/decisions,
recent Claude Code session tails), `file_claim`/`file_release`
(refuses if another *live* agent — tracked via a 300-second
heartbeat TTL — already holds the claim), `note_add` with
`kind: handoff|finding|decision` and an explicit `scope: global|workspace|project`
(workspace is the default; cross-workspace visibility must be deliberate,
never silent bleed, per the README's own scoping rules). State is
append-only JSON under `.ozone-context/`, which is git-visible — i.e. it
has a real, human-readable, `git log`-able audit trail, the same one this
very guide's work is being recorded into via this pass's own `note_add`
call.

### 3.3 What would actually be needed to extend this pattern to two real, separate hosts

If the goal were genuinely "connect two separate Ozone-Studio deployments
in two jurisdictions" using this same MCP pattern (not just within one
shared repo clone), one of two real changes would be required — there is
no third option that keeps today's local/stdio design as-is:

**(a) Each Ozone-Studio instance exposes its own MCP server over a
reachable network transport instead of stdio, with its own auth.** This
is a real, buildable option — MCP supports HTTP/SSE transports, not just
stdio — but it means standing up a new network-facing service with its
own authentication, its own encryption-in-transit story, and its own
firewall/port exposure decisions. Concretely, this duplicates work the
HTTP API (Ed25519 challenge-response auth, `src/auth/mod.rs`) and the P2P
layer (noise encryption, yamux multiplexing, already compiled into the
same binary) already do. Building this would mean building a second,
parallel secure-transport implementation to do a job the product already
ships one for.

**(b) `.ozone-context/` is synced between hosts via `git push`/`pull`,**
relying on a git remote both sides can reach. This is a real, working
pattern for some purposes — it would give both hosts eventually-identical
note/claim history — but it is **store-and-forward/batch, not live**: a
`file_claim` refusal (the core "don't edit what another live agent holds"
guarantee) depends on seeing the *current* claim state, and a claim made
on Host A is invisible to Host B until the next push/pull cycle. The
300-second presence-heartbeat TTL that makes "live" meaningful within one
shared filesystem has no real analogue across an asynchronous git-sync
lag of arbitrary length. It also requires a git remote reachable by both
sides, which is itself a cross-jurisdiction network dependency no
different in kind from what P2P already solves directly.

Either path is a real engineering project layered on top of what exists
today — neither is "just use the MCP server that's already there."

---

## 4. The operator's actual question, answered directly

**For two separate Ozone-Studio host instances specifically — is a
shared-MCP-style approach the best fit, or is native P2P the better fit?**

**Native P2P is the better-fit mechanism, and the recommendation is to
fix and use it, not to extend the MCP pattern into a second host-to-host
channel.** The reasoning:

- MCP shared-context's local/stdio/shared-filesystem design is a
  **structural assumption**, not a bug — it is exactly what makes it
  simple, zero-dependency, and already working today for multi-agent-on-
  one-repo coordination. Removing that assumption to serve a different
  job (two hosts, two filesystems, two jurisdictions) is not a small
  patch; per §3.3 it means building a second transport-and-auth stack
  that duplicates what the binary already ships.
- The P2P layer's two open problems (§2.3) are **bugs in a design that
  was always aimed at this exact job** — host-to-host, cross-jurisdiction,
  already using real encryption (noise) and multiplexing (yamux), already
  has the domain-level sync-priority logic wired end-to-end from live
  graph events to the sync queues. Fixing "call `listen_on`" (done, in
  the working tree now) and "poll the swarm's event loop" (still open) is
  bounded, well-understood work inside the layer that was built for this
  — not a redesign.
- If the operator's parenthetical — "or even just between AI coding
  agents" — means two agents working on **one shared repo/filesystem**,
  that is precisely the shared-MCP pattern's proven, current, real use
  case (this very session). No change is needed there; it already works.
  But if it means two agents (or two Ozone-Studio instances) that do
  **not** share a filesystem — across the local-machine/EU-VPS boundary
  this session is actually testing — then the filesystem assumption in
  §3.1 is exactly the thing that doesn't hold, and P2P (or option (a)/(b)
  in §3.3, both of which re-derive pieces of what P2P already has) is
  required either way.

In short: use native P2P for instance-to-instance, cross-jurisdiction
sync of domain knowledge (methodologies, blueprints, findings) — it is
the mechanism actually designed for that boundary, with real bugs but no
structural ceiling. Keep shared-MCP coordination exactly where it already
excels: multiple AI agents working on one shared repo clone.

---

## 5. Native benefits of each approach, concretely

**Native P2P (`src/network/mod.rs`):**
- Built into the product binary itself — any deployed instance has it
  with zero extra process, port, or dependency beyond the binary already
  running.
- Transport-level encryption (noise) and multiplexing (yamux) are part
  of the design today, not a future addition.
- Designed explicitly to cross real network and jurisdiction boundaries
  — dials real multiaddrs over TCP, not bound to a shared filesystem.
- gossipsub pub/sub naturally fans one write out to many peers without
  the sender having to enumerate them individually.
- mDNS gives automatic local-network peer discovery for free when
  instances share a LAN.
- Sync-priority logic (methodologies/blueprints always, findings
  batched, pipeline results on-demand, user data never) is already real
  and wired from live graph events through to the sync queues — the
  remaining work is specifically at the transport-connection layer, not
  the domain logic.

**Shared MCP coordination (`tools/ozone-shared-context/`):**
- Human-auditable history — `.ozone-context/` is plain git-visible JSON;
  `git log` is a real audit trail, not a log format that needs a special
  viewer.
- Structured claims prevent concurrent-edit collisions directly — a
  `file_claim` genuinely refuses when another live agent holds the file,
  rather than relying on agents to coordinate by convention alone.
- Zero-dependency — plain Node, stdio, no new ports or firewall rules to
  open or reason about.
- Already proven working today, for its actual intended use case
  (multiple AI agents on one shared repo clone) — unlike P2P's current
  open transport bugs, this has been running correctly all session.
- Scope discipline is explicit and enforced by convention — `global` vs
  `workspace` vs `project` note scoping means cross-workspace visibility
  is a deliberate choice, never silent bleed.

**Device pairing (`src/pairing.rs` / `src/auth/mod.rs`):**
- Simplest and narrowest case of the three — one host, the operator's
  own devices — and it is already fully working end-to-end.
- QR/possession-based UX fits personal devices well: no password to type
  or key to copy, just "scan this with the phone you already have in
  hand."
- Reuses the exact same session store and expiry policy as the raw
  Ed25519 challenge-response login path — one real session mechanism,
  not two parallel ones to keep in sync.

---

## 5.5. A fourth pattern this guide missed on its first pass: exposing P2P *control* operations as MCP tools

The operator asked directly whether P2P could "be an MCP and a Tool,
run through the MCP, ensuring cleaner and better consistency with
shared context across these harnesses" — and asked for a review to make
sure nothing was being dropped. It was: §4's recommendation answers a
narrower question than the operator actually asked. §3–4 only evaluate
whether MCP should **replace** P2P's data plane (the actual encrypted
sync of methodology/blueprint/finding bytes between hosts) — correctly
concluding no, since that would re-derive transport and auth the binary
already ships (§3.3). It does not evaluate a separate, complementary
idea: wrapping P2P's existing **control** operations — connect to a
peer, read status, (later) trigger a broadcast — as MCP tools, the same
way `file_claim`/`note_add` already wrap `.ozone-context/` state.

**This is a real, low-risk, additive idea, distinct from §3.3's options
(a)/(b), and it does not conflict with §4's recommendation — it sits on
top of it.** Concretely:

- Add a handful of thin tools (e.g. `ozone_p2p_status`,
  `ozone_p2p_connect_peer`) to the shared-context MCP server (or a
  sibling one) that do nothing more than call the Ozone-Studio HTTP API
  that's already there — `GET /health` for `peer_count`/`p2p_enabled`,
  `POST /network/connect-peer` for dialing. Both of these are
  loopback-only (`127.0.0.1:50051`), exactly the same host the MCP
  server itself runs on — so this needs **zero new network exposure,
  zero new auth design, zero new firewall decisions**. It is pure
  ergonomics over infrastructure that already exists, the same relationship
  `ozone-shared-context`'s existing tools already have to
  `.ozone-context/`.
- The concrete benefit, in the operator's own words — "cleaner and
  better consistency with shared context across these harnesses" — is
  real: any MCP-speaking harness (Claude Code, ZCode, a future one)
  gets one typed, discoverable way to drive and observe P2P state,
  instead of each harness independently reaching for raw HTTP/curl.
  This session's own P2P verification work is the live example of the
  gap: every connect-peer call and every `/health` check this pass was
  done by hand over `curl` through manually-built SSH tunnels — exactly
  the repetitive, bespoke-per-harness work a thin tool wrapper removes.
- It composes naturally with the existing `note_add` audit trail: a
  `ozone_p2p_connect_peer` tool could log its own outcome as a `finding`
  note automatically, giving one unified, git-visible record of both
  agent-coordination activity and instance-to-instance P2P activity —
  not two disconnected logs.
- What this does **not** do: it doesn't make MCP the transport for the
  actual cross-host sync traffic (still native P2P, per §4's
  unchanged recommendation), and it doesn't solve cross-*machine* MCP
  access either — the tool still only ever talks to the one
  Ozone-Studio instance reachable from wherever the MCP server itself
  is running (the same real constraint §3.3 found for a hypothetical
  network-exposed MCP server, just avoided entirely here by staying
  loopback-only and letting the operator's own existing SSH-tunnel
  pattern bridge machines, the same way it already does for pairing a
  UI device).

**Status: not yet built.** This is a concrete, scoped follow-up
recommendation — new tools in `tools/ozone-shared-context/server.js` (or
a sibling MCP server) wrapping existing local HTTP routes — not a
change to this guide's core conclusion in §4.

---

## 5.6. Should an Ozone-Studio instance itself be a participant in shared-context coordination, not just a mirror target?

A third, distinct question from §5.5's: not "can P2P's control routes be
called through MCP," and not "should MCP replace P2P's data-plane sync"
(§3–4, still no) — but **should the Ozone-Studio host process itself
(local instance, and separately the EU VPS instance) become an active
*party* in the same `context_summary` / `file_claim` / `note_add` /
presence graph that Claude Code and ZCode already use to coordinate with
each other, rather than a thing their coordination state is only
mirrored *into* for display?**

### 5.6.1 The real current state — verified this pass, not assumed

This repo's own `tools/ozone-shared-context/README.md` describes a "next
layer (designed, not yet wired): mirror notes/claims as ZSEI containers
under the SharedContext root... Blocked only on the container-over-HTTP
shape." **That README is stale relative to the real code.** Reading
`src/context_mirror.rs` directly shows this is not a design note — it is
a complete, tested module:

- `context_mirror::mirror()` turns any `{kind, agent, title, body, files,
  scope}` event into a real `ContainerType::CoordinationEvent` ZSEI
  container under `/SharedContext` (`SHARED_CONTEXT_ROOT_ID = 8`), with
  scope keywords (`scope:global` / `ws:<id>` / `proj:<id>`), a
  `file:<path>` keyword per claimed file, a `claim:<path>` dedupe key so
  re-claiming a file reuses one container instead of spamming the graph,
  and the full event body persisted to
  `<data_dir>/shared_context/<key>.json` via the same content-pointer
  pattern the jurisdiction rule sets use. Five tests cover this
  (`T-CO1`–`T-CO5`), including a real concurrency-collision regression
  test (`concurrent_same_agent_same_kind_mirrors_never_collide`).
- `POST /context/mirror` (`mirror_context`, `src/grpc/mod.rs:4047-4068`)
  is a thin wrapper calling `mirror()` directly — and it is already the
  live target of `tools/ozone-shared-context/server.js`'s own
  `mirrorContext()` function, which fires on every `file_claim`,
  `file_release`, `note_add`, and `task_create` the Node server handles
  (confirmed by reading `server.js` — `mirrorContext()` posts to
  `/context/mirror`, or to `/mcp/call` with `tool: "context_mirror"`
  first when `OZONE_THROUGH_HOST=1`, falling back to the direct POST on
  any failure).

So CC/ZCode's coordination events already become real, queryable graph
containers today — the "container-over-HTTP shape" is built and working,
not blocked. What genuinely *is* missing (the part the README's framing
actually should have pointed at) is the deeper layer one sentence further
in the same README note — real **graph relationships** between these
events (e.g. an edge linking a `claim` container to the container
representing the claimed file, or a release back to its claim): every
`mirror()`-produced container hard-codes `relationships: vec![]`
(`src/context_mirror.rs:243`), confirmed by grep to be the only place
`CoordinationEvent` containers are ever constructed. `src/mcp_graph.rs`
does build real typed relationship edges, but only for a different
path — a tool's own `graph` output block — never for coordination events.
Today's CoordinationEvent containers carry flat keyword tags for search,
not graph edges.

**The direction of participation today is one-way, but not in the way
"mirror" alone suggests — there are two distinct surfaces, and the host
already writes to one of them:**

1. `.ozone-context/state.json` + `notes.jsonl` — the canonical store
   `context_summary`/`file_claim`/`note_add`/`presence_heartbeat` read
   and write. This is owned **exclusively** by Node `server.js`
   processes — one per agent session (CC's via `.mcp.json`, ZCode's via
   `.zcode/config.json`). The host never writes to it. It only
   **reads** it, read-only, for display: `GET /coordination/presence`
   and `GET /coordination/claims` (`src/grpc/mod.rs:4071-4082`, whose own
   comment says so explicitly: "a read-only view of its state, not a
   replacement for it — writes still go through that server's own
   tools").
2. The `/SharedContext` ZSEI graph root — populated by `context_mirror
   ::mirror()`. CC/ZCode's events reach it via server.js's HTTP push
   (above). But the **host's own internal events already reach the exact
   same graph root too**, via three confirmed call sites that invoke
   `context_mirror::mirror()` directly, in-process — no MCP, no stdio,
   no network hop: a `config_change` event on every successful
   `/config/set` (`src/grpc/mod.rs:2155-2169`), a `tool_call` ripple on
   every `/mcp/call` (`src/grpc/mod.rs:3872-3899`), and of course
   `mirror_context` itself.

**Confirmed, and this is the central finding**: the host cannot become a
participant in surface (1) through its own existing MCP abstraction even
if asked to. `McpRegistry::invoke()` (`src/mcp.rs:505-514`) has an
explicit branch for stdio-transport tools — which is exactly how
`ozone-shared-context` registers itself — and that branch does not call
the tool at all:

```rust
McpTransport::Stdio => {
    // Stdio tools belong to a live agent session — invocation is
    // that agent issuing the call on its own transport.
    Ok(serde_json::json!({
        "success": true,
        "delegated": true,
        "tool": call.tool,
        "message": "stdio tool — invocation delegated to the owning agent session",
    }))
```

This is deliberate, documented architecture, not an oversight: a test
(`invoke_stdio_tool_delegates_and_meters`, `src/mcp.rs:622-649`) exercises
exactly this path with `tool: "shared-context"` and asserts the
delegation response. The host has no stdio client of its own for this
tool and today's `McpRegistry` is built to explicitly refuse to pretend
it does. So: today, Ozone-Studio writes its *own* activity into the same
graph destination CC/ZCode's coordination lands in (surface 2), but it
has no voice at all in the actual `context_summary`/`file_claim`/
`note_add` protocol (surface 1) that CC/ZCode use to coordinate with
*each other* — it is a parallel writer into a shared destination, not a
third party in the same conversation.

### 5.6.2 Real native benefits this would add

- **Closes a real, concrete blind spot.** `context_summary` — the one
  call CC/ZCode already make at the start of every session — reads only
  surface (1): other AI agents' presence/claims/notes. It has zero
  visibility into surface (2), even though the host's own methodology
  creation, blueprint creation, orchestration runs, and tool calls are
  *already* being recorded as CoordinationEvent containers in the same
  graph root, right now. A dev agent editing code today has no way to see
  that the live running system is concurrently orchestrating a run that
  touches the very containers its edit will affect — the two activity
  streams exist side by side in the same graph and are simply never
  read together. Making the host a genuine participant — at minimum,
  surfacing its own already-recorded graph activity back through
  `context_summary` — closes this without inventing new data, since the
  data already exists.
- **Free cross-host reach, conditionally.** Any CoordinationEvent
  container creation (claims, notes, or a future host participation
  event) is an ordinary ZSEI `CreateContainer`, which
  `src/zsei/mod.rs:228` unconditionally emits through
  `graph_events::emit` for every container type, not just specific ones.
  `orchestrator::amt_loop::spawn_network_hook_bridge`'s own match
  (`src/orchestrator/amt_loop.rs:1720-1729`) has exactly three arms —
  `"Methodology"`, `"Blueprint"`, and a catch-all `_` that calls
  `on_container_created` for everything else, including
  `"CoordinationEvent"`. `on_container_created` (`src/network/mod.rs:
  503-517`) builds a `SyncItemType::Container` / `SyncPriority::Normal`
  item — precisely the module doc's own "New Container Findings: ALWAYS
  sync (queue for batch)" category. Verified by reading the real match
  arms and the real `SyncPriority` assignment, not assumed: **a local
  coordination finding that becomes a real ZSEI container already rides
  the exact same existing findings-sync pathway any other new container
  does** — no new git-sync, no second network-exposed MCP server needed.
  This is conditional, not live today (see 5.6.3).

### 5.6.3 Real limits — local and EU-VPS participation are not the same case

- **Local instance**: genuinely the easier case, and largely already
  true in one direction — host and MCP server already share one
  filesystem/machine, which is exactly why `context_mirror::mirror()`
  can be called in-process with zero network hop. Closing the remaining
  gap (host reading/contributing to surface (1), not just writing its
  own events to surface (2)) is an in-process, same-machine change.
- **EU VPS instance**: a separate process, on a separate machine, with
  its own `.ozone-context/` (or none) and its own ZSEI graph. Do not
  conflate it with the local case. For its own activity to reach the
  *same* `context_summary` a dev agent checks on the local machine, a
  real cross-machine transport is unavoidable — this runs into exactly
  the constraint §3.3 already documented (a network-exposed MCP server
  duplicating P2P's transport/auth, or batch git-sync of
  `.ozone-context/` that loses the live claim-refusal guarantee) —
  **unless** it rides the P2P findings-sync path in 5.6.2, which is
  itself still pending: Bug 2 from §2.3 (the libp2p `Swarm`'s event loop
  never polled) is confirmed still open this session, being worked on
  elsewhere. Until that lands, no mechanism — MCP, git-sync, or P2P —
  actually gets a local coordination finding to the EU instance live
  today. Don't read 5.6.2's "free cross-host reach" as already working;
  it is real, verified wiring sitting idle behind a real, separately
  tracked bug.

### 5.6.4 A concrete, scoped first step — **not yet built**

Building a literal MCP stdio client inside the Rust binary (spawning a
`node server.js` child process and speaking its JSON-RPC protocol) would
be real but heavier than this job needs, and inconsistent with how this
session has otherwise extended the host additively — e.g. `POST
/orchestration/node-event` (`src/grpc/mod.rs:1373-1384`,
`src/orchestration_events.rs`), a thin new HTTP endpoint fed by whatever
needs to report in, no client library on either side. The same shape
fits here better than a new stdio client, and the host is already most
of the way there:

1. The host already has a working, in-process "report a coordination
   event" primitive — `context_mirror::mirror()` — used today for
   `config_change` and `tool_call`. The first step is simply calling it
   from more of the host's own real activity that already flows through
   `GraphEventHub` (methodology/blueprint/container creation,
   orchestration-run start/completion) — no new mechanism, just more
   call sites of one that already exists and is tested.
2. Add one new read-only HTTP endpoint, matching the existing
   `/coordination/presence` / `/coordination/claims` read-only-peek
   convention, that returns recent CoordinationEvent containers from the
   graph (e.g. `GET /context/recent-host-events`).
3. Extend `server.js`'s `contextSummary()` with one more best-effort,
   non-blocking `fetch` to that endpoint (the same pattern its existing
   `pushActivity`/`mirrorContext` calls already use — `.catch(() => {})`,
   never blocking local coordination on the host being reachable), so a
   single `context_summary` call surfaces both surfaces side by side.

This is additive on both ends, reuses code that already exists and is
tested, and does not require a stdio client, a second coordination
store, or any change to the EU-VPS case in 5.6.3. **None of this is
built yet** — it is a scoped recommendation, not a status claim.

---

## 6. Summary table

| Question | Device pairing | Native P2P | Shared MCP context |
|---|---|---|---|
| Is it currently crossing a real network/jurisdiction boundary? | No (same host) | Yes, by design — two confirmed open transport bugs as of this pass | No — stdio + local filesystem only |
| Does it need new infrastructure to do its actual job? | No | No (compiled in) | No, for its actual job (one repo clone). Yes, for host-to-host (§3.3) |
| Proven working today? | Yes | Domain logic yes; transport connectivity no (§2.3) | Yes, in continuous live use |
| Right tool for host-to-host sync? | No (not its job) | **Yes — recommended** | No — would require re-deriving transport/auth P2P already has |
| Right tool for multi-agent-on-one-repo coordination? | No (not its job) | No (wrong granularity — it's for domain-knowledge containers, not file-edit claims) | **Yes — already the right tool, already working** |

---

## 7. Trust and verification for incoming P2P sync content — a real, currently-open gap

**Status update since §2.3 was written**: all remaining transport bugs
named in §2.3 are now fixed and the pipeline has been proven live,
end-to-end, bidirectionally — a real Methodology container created on
the local instance was received and logged by the EU VPS instance 93
seconds later (gossipsub subscribe + a message handler were the two
final missing pieces, on top of `listen_on`/the event-loop poll/the
0-second idle timeout). That makes this section's gap live and current,
not hypothetical: **right now, any peer that completes a libp2p
handshake with this instance has its messages accepted and logged as
real signal, with zero check against anything the operator has
approved.**

The operator's concern, direct: sync as it stands fits *paired* devices
— something the operator explicitly approved — but P2P peer connections
today have no such concept at all. This section investigates the two
existing review-like mechanisms in this codebase (the zero-shot call
registry's expansion pattern, and the methodology/AMT background loops)
to see whether either is a usable template for closing that gap, and
proposes a concrete, honestly-scoped design. **Nothing in this section
is implemented — it is a design write-up for an operator decision**,
following the same caution this session applied to every other
auth/trust-boundary question.

### 7.1 The real gap, stated plainly

Verified by reading the real code this pass, not assumed:

- **"Paired" and "P2P-connected" are two subsystems with zero
  cross-reference.** `src/pairing.rs` / `src/auth/mod.rs` /
  `src/types/auth.rs` (`DeviceRegistration`, `Session`, `AuthSystem`)
  never mention `PeerId` anywhere (grepped `src/auth/`, `src/pairing.rs`,
  `src/types/auth.rs` for `PeerId` — zero hits). `src/network/mod.rs`'s
  `PeerInfo`/`known_peers` never mention device pairing, a user, or a
  session anywhere either. These were built as, and remain, completely
  separate trust domains.
- **Every connection path inserts the peer unconditionally.** The
  background Swarm-poll task's `ConnectionEstablished` arm
  (`src/network/mod.rs`, around line 325) inserts the new peer into
  `known_peers` with `reputation: 1.0` the instant the libp2p handshake
  completes — no check of any kind. `connect_peer` (same file, around
  line 659) does the same on the dialing side with `reputation: 0.5`.
  Both paths are reachable by anyone who can address this host's P2P
  port (loopback-bound today, per §2.3, reached in practice over an
  operator's own SSH tunnel — but the *code* enforces nothing; the
  operator's tunnel topology is the only thing standing in for a trust
  boundary right now).
- **The gossipsub message handler accepts and logs anything from
  anyone.** The `Gossipsub(Event::Message{propagation_source, message,
  ..})` arm (same file, around line 403) does exactly one thing: decode
  the bytes as UTF-8 and `tracing::info!` them as "received sync
  message from {peer}: {summary}" — no check of `propagation_source`
  against any approved/paired list, no validation of the envelope shape
  beyond what downstream log-reading happens to tolerate.
- **Scope of what's actually at risk today, stated honestly**: the
  envelope this handler logs is a lightweight finding signal
  (`container_id`/`container_type`/`parent_id`/`source`/
  `scope_keywords`) — not full container content. Full content
  replication does not exist anywhere in this codebase (confirmed
  earlier this session, unchanged). So today's actual exposure is "an
  unapproved peer's claims get written into this instance's logs as if
  they were real system activity," not yet "an unapproved peer can
  inject fabricated methodology/blueprint content" — but the trust gate
  that's missing is exactly the gate that would also be needed the
  moment real content sync is ever built on top of this same connection.
  Fixing it now, while the blast radius is small, is cheaper than
  retrofitting it onto a bigger feature later.

### 7.2 A trust gate: who gets to connect/sync at all

**Field(s) needed, grounded in what's already there**: `PeerInfo`
(`src/network/mod.rs`) is structurally close to what's needed — it's
already a per-`PeerId` record — it is just missing the one thing that
matters: `paired: bool` (default `false`) and `paired_at: Option<u64>`.
Both are additive `struct` fields; every existing `PeerInfo`
construction site would need updating to set `paired: false` by
default, which is the safe default (a peer is untrusted until
explicitly approved, not trusted-until-revoked).

**A real wrinkle this session already found and must not be
re-introduced here**: `known_peers` is in-memory only — confirmed
earlier this session that a restart loses all peer state. A `paired`
flag living only inside `PeerInfo` would not survive a restart either,
which would silently un-pair every peer on every restart — worse than
today's "no concept of pairing at all," since it would look like a real
security control that quietly resets. **The trust decision needs its
own small, durable store** — e.g. `paired_peers.json` (`{PeerId: {name,
paired_at}}`), same append/persist convention as
`methodology_gaps.json`/`amt_reexpansion_candidates.json` elsewhere in
this codebase — loaded at boot and consulted (not just the in-memory
`PeerInfo.paired` mirror) whenever a connection or message needs a real
answer.

**UX for the pairing action itself — recommend the simpler of two real
options, not the heavier one:**

- **Option A (recommended): a new operator-only endpoint, e.g. `POST
  /network/approve-peer {peer_id, label}`**, that writes an entry to
  `paired_peers.json` and sets the matching `known_peers` entry's
  `paired`/`paired_at`. This mirrors the exact operational pattern this
  session already used by hand all pass — `connect_peer` is itself a
  one-shot operator HTTP call with no further ceremony — so
  `approve-peer` is the same shape of action, not a new paradigm.
- **Option B: extend the QR/pairing-code flow (`src/pairing.rs`) to
  cover P2P peers too.** Considered and **not recommended as the first
  version**: that flow's entire design — a short-lived code, a phone
  scanning a QR, a human tapping "approve" — is built around a *person*
  approving *their own device* in real time. A P2P peer is a second
  Ozone-Studio *instance*, not a personal device with someone holding it
  up to a camera; there is no natural "phone in the loop" for
  host-to-host pairing. Forcing peer approval through the QR/session
  machinery would mean inventing a parallel meaning for "device" and
  "session" that doesn't fit what a libp2p `PeerId` actually is (a
  cryptographic identity already generated from its own Ed25519
  keypair, per `NetworkManager::new`'s `identity::Keypair::
  generate_ed25519()` — the trust anchor already exists, it's just
  never been marked approved anywhere). Option A reuses that existing
  identity directly, at the cost of being a simpler, one-sided
  "operator says yes" action rather than a mutual handshake — an
  honest trade worth naming, not hidden: Option A doesn't protect
  against an operator mistakenly approving the wrong `PeerId` (e.g. a
  copy-paste error), whereas a two-sided challenge/code-confirm flow
  (closer to Option B, bigger to build) would. For a first version,
  where the operator is manually running both instances and already
  copy-pastes multiaddrs by hand to call `connect_peer`, Option A's risk
  is small and symmetric with a risk that already exists today in
  `connect_peer` itself.

**Where the gate actually checks**: recommend gating at the **message-
acceptance point** (the gossipsub `Message` handler) rather than at
the connection/handshake point, as the first version. Concretely: when
a message arrives, look up `propagation_source` in the paired store;
if paired, proceed as today (log it as real signal, and — once 7.3's
review exists — feed it there); if not paired, log at `debug` (not
`info`) with an explicit "message from unpaired peer {id}, ignored"
note — visible if someone looks, never silently either trusted or
invisibly dropped. Refusing the TCP/noise/yamux handshake itself for an
unpaired peer is a more aggressive second step (it would mean mDNS
auto-discovery and `connect_peer` dials from a paired operator could no
longer even establish a raw connection to confirm reachability) —
worth deciding separately, not bundled into this first version.

### 7.3 A content review gate, modeled on meta_loop/amt_loop's real mechanics

**What the real mechanics actually are** (read in full this pass,
`src/orchestrator/meta_loop.rs` and `src/orchestrator/amt_loop.rs`, not
just their doc comments): both loops wake on an interval (or, for
`amt_loop`, an event-driven `Notify` with the interval as fallback),
load a **persisted** candidate/gap log, process a **bounded batch per
pass** (`MAX_GAPS_PER_PASS` / `MAX_CANDIDATES_PER_PASS = 12`, leftovers
explicitly carried to the next tick, never silently dropped), and for
each item: (1) run a **duplication check** against the real store
(`search_by_keywords`, with an extra filter excluding already-retired
placeholder shells) — if something already covers it, mark handled and
skip, no model call spent; (2) if not covered, make **one** zero-shot
call with a **persisted retry-attempt counter** on the record itself
(`MAX_DRAFT_ATTEMPTS` / `MAX_REEXPAND_ATTEMPTS = 10`, escalating through
a configured model-fallback chain on each retry, never clamping to one
broken model forever); (3) **detect confetti** (multiple conflicting
JSON candidates in one response) and treat it as a retriable failure,
never a guess; (4) **reject placeholder/shell content** explicitly
(empty rules, literal `"..."` fields) rather than persisting it; (5)
only a response that is well-formed, non-duplicate, and substantively
real gets **persisted** (a real `methodology_id`, or a real tree
mutation written back to disk) — every other outcome is recorded as
`handled`/`failed_after_max_attempts`/`failed_structural` on the
record, never silently re-tried forever and never silently accepted.

**What would need to change/be added to adapt this to "a peer sent us a
container — should we trust and incorporate it?"**

1. **The trust gate (§7.2) has to run first and is not optional** — see
   §7.4 for why a model call cannot substitute for it.
2. **The duplication check transfers directly, almost unchanged.** The
   received envelope already carries `scope_keywords`/`container_type`;
   running the exact same `search_by_keywords`-based "do we already have
   an equivalent container" check meta_loop uses before ever trusting
   the signal as new is a close, honest analogue — same question
   (has this already been covered?), same mechanism, same store.
3. **The genuine judgment question is different in kind, and that
   matters.** meta_loop/amt_loop draft or deepen content the system
   itself is originating — the model is asked "is this worth creating,"
   never "should I believe this came from where it claims to come
   from." Incoming peer content is a provenance/adoption question, not
   a creation question. There is no existing prompt anywhere in this
   codebase that asks "does this external claim look trustworthy" —
   one would have to be written from nothing, and its answer would not
   be a security boundary (see §7.4) — at most a content-quality signal
   layered **on top of** a peer that's already passed §7.2's trust gate.
4. **A first, small, real version — honestly scoped**: today the
   message is a lightweight finding signal, not full content (§7.1) —
   there is no actual container content to "simulate and verify" yet,
   only a claim that one exists elsewhere. So the right-sized first
   build is NOT a full content-verification pipeline; it's the same
   *shape* meta_loop/amt_loop already use, applied to this signal: on
   receipt from a **paired** peer, append the envelope to a small
   persisted log (e.g. `peer_sync_candidates.jsonl`, same convention as
   `methodology_gaps.json`), run a bounded periodic (or `Notify`-woken,
   like `amt_loop`'s pattern) pass that runs the duplication check
   (point 2 above) and, if not already covered, surfaces it as a real,
   durable, **human/operator-reviewable** item — the same role
   `decision_review.rs`'s `ReviewPending` already plays elsewhere in
   this codebase: a real, revisitable record, not an auto-created
   mirror and not a silently dropped line. No model call needed for
   this first version at all — it is pure "don't trust it, and don't
   lose it either" bookkeeping, which is itself the actual, real gap
   today (currently it's neither reviewable nor gated — just logged).
5. **Bigger/future work, explicitly not this version**: once (if) full
   container-content replication is ever designed and built — a
   separate, larger feature that doesn't exist today — a genuine
   zero-shot review call (same pipeline-9 contract as everything else
   in this codebase) could assess an incoming container's actual
   content the way meta_loop assesses a drafted methodology: well-
   formed, substantive, non-duplicate, no scope/jurisdiction conflict.
   That call is realistic to build using this exact pattern once there
   is real content to point it at — it is not realistic, or honest, to
   design in detail against a content-sync feature that doesn't exist
   yet. Sequencing matters: content replication is the prerequisite,
   not something to backfill around a review mechanism built first.

### 7.4 Is the zero-shot call registry/expansion pattern actually a good template here?

**Partially — it answers a related but different question, and forcing
the analogy where it doesn't hold would be dishonest.** What the
registry/expansion guide actually describe (confirmed by reading both
in full this pass): a catalog-and-doctrine for adding **new judgment
calls on top of decisions this system already makes about its own,
already-trusted, already-owned content** — file-role classification,
relationship-confidence borderline cases, branch-coverage
reconciliation, confetti-candidate adjudication. The explicit,
operator-directed **"review-before-judge" rule** (2026-10-08) is the
single most relevant piece: before reaching for a new zero-shot call in
response to any detected anomaly, first check whether a plain,
mechanical, fixable defect explains it, and only build judgment-call
infrastructure once that's ruled out.

**What genuinely transfers:**
- **The review-before-judge discipline itself, applied correctly here**:
  the operator's instinct to ask "should the meta-loop pattern review
  this" is exactly right to ask — and applying the registry's own
  doctrine to itself says the actual missing piece is a **plain,
  mechanical, structural fix** (§7.2's pairing gate), not a model call.
  Reaching straight for an LLM-based adjudicator over unauthenticated
  peer content, before a trust gate exists at all, would be the exact
  mistake the doctrine was written to prevent — building judgment-call
  infrastructure around a gap a direct fix closes more cheaply and more
  certainly.
- **The reusable shape** — persisted candidate log, bounded periodic
  pass, dedupe-before-acting, structured accept/reject outcome, nothing
  silently dropped or silently trusted — transfers cleanly, because
  it's the same shape meta_loop/amt_loop already use (§7.3 above reuses
  it directly).

**What does not cleanly transfer, and why:** every real zero-shot call
in the registry judges something the system already has full access to
and already trusts the source of (its own file, its own prior
generation, its own AMT branch). The open question for incoming P2P
content isn't "is this judgment/classification good" — it's "should
this source be believed at all," which is an **authorization**
question, not a content-quality question. No prompt asking a model
"does this peer/content seem trustworthy" is a real security boundary —
a model can be as easily fooled by a convincing fabrication as a human
skimming a log line, and unlike every registry call site, there is no
already-trusted ground truth available to check it against (that's the
entire problem). The trust decision has to be a real, operator-approved
fact (§7.2), not an inference. Once that's established, the registry's
pattern is a legitimate second-layer content-quality check (§7.3 point
5) — but it cannot be, and should not be mistaken for, the trust gate
itself.

### 7.5 Is project/workspace/linked-file sync a thing today?

**No — plainly, for paired devices and P2P peers alike.** Already
confirmed this session and reconfirmed here: `SyncItemType`
(`src/network/mod.rs`) has exactly four variants — `Methodology`,
`Blueprint`, `Container`, `PipelineResult` — there is no
Project/Workspace/FileReference category anywhere in the sync pipeline,
for anyone, paired or not. This isn't a gate that's missing a pairing
check; the category doesn't exist in the type system at all.

**The real current P2P sync scope** (restated accurately, cross-
referencing §2.2 above rather than re-deriving it): Methodologies and
Blueprints **always** sync (no significance check); new Container
findings **always** sync, batched (and, per §7.1, confirmed this pass
to be a lightweight metadata signal only — `container_id`/
`container_type`/`parent_id`/`scope_keywords` — not full content);
Pipeline Results sync **only on-demand**, by explicit request; User
Data **never** syncs. If project/workspace/linked-file sync is wanted
in the future, it would require a new `SyncItemType` variant added
deliberately — and, per this section, the trust gate (§7.2) and review
gate (§7.3) proposed above should apply to it from day one, since
project/file content is exactly the kind of content-bearing sync this
whole section exists to guard.

### 7.6 Status

**Nothing in this section is built.** §7.2's `PeerInfo`/`paired_peers.json`
fields, the `/network/approve-peer` endpoint, and §7.3's
`peer_sync_candidates.jsonl` review pass are all design proposals for
an explicit operator decision — consistent with this session's standing
rule that trust/security architecture changes (same category as the
`OZONE_ZSEI_REQUIRE_SESSION` question) get flagged for sign-off, not
built unilaterally.
