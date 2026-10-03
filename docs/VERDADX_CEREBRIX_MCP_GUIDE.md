# VerdadX + CereBrix — full review and MCP strip/integration guide

> Eleventh doctrine doc. Operator directive (2026-09-29): full review of two
> external projects — VerdadX-Systems-Linux ("the linux hardener") and
> CereBrix — **no edits, review and guide only**. This doc captures the
> real state of both, honestly, and documents (does not execute) a plan
> for stripping each down into MCP tools that plug into Ozone-Studio's
> existing registry/graph architecture. Companion to: MODALITY_EXPANSION_
> GUIDE.md (§2 EEG-108 status, §3 the physical-bridge pattern this reuses),
> LOCALIZATION_TRAVERSAL_GUIDE.md.

---

## 1. Scope & method

Both projects live outside this repo (`~/Projects/VerdadX/linux`,
`~/Projects/CereBrix`) — not part of Ozone-Studio's crate tree, no
`file_claim` needed on their own files (only this guide doc, in-repo, is
claimed). Method: read real source directly (not the projects' own partial
AI-generated breakdown docs, which were incomplete/malformed where
checked), confirm claims against actual file content and, where possible,
a real `cargo check`. Nothing in either project was modified.

---

## 2. VerdadX-Systems-Linux — full review

### 2.1 What it is

A from-scratch Rust/Tauri desktop app: "a next-gen, modular Linux security
distribution... behavior-based detection, advanced sandboxing, real-time
logging, and integrity monitoring" (its own README). Real scope, real
ambition: kernel-level secure boot/UEFI verification, a custom Mandatory
Access Control (MAC) engine, a behavior-anomaly detection engine, process
sandboxing, package analysis, an AI prediction engine, and a logging/alert
subsystem — composed by one `SecuritySystem` struct that's supposed to own
and start all of them together.

**Real size**: 15,555 lines across `src-tauri/src/` (66 files). Frontend
(`src/`) is a matching React/TS app with its own SecurityMonitor/
PackageAnalysis/SandboxMonitor dashboards — real, well-structured
components that already expect the shape of data the backend should (but
doesn't yet) provide.

### 2.2 Does it compile? No — 5 independent, confirmed reasons

Checked directly (`grep`, `cargo check`), not assumed:

1. **`src-tauri/src/lib.rs`**: `SecuritySystem::new_with_config` builds and
   returns `Self { kernel_security, behavior_engine, sandbox, mac,
   integrity, logger, ai_engine, package_analyzer, config }` — but
   `pub struct SecuritySystem` is **never declared anywhere in the
   codebase** (`grep -rln "SecuritySystem" src/` matches only `lib.rs`
   itself). Real, unconditional compile error.
2. **Same function**: binds a local `let package_monitor = Arc::new(...)`
   then references an undefined `package_analyzer` in the struct literal.
   A second, independent unresolved-name error in the same ~15 lines.
3. **`sandbox/network.rs`**: `use std::net::MacAddr;` — `MacAddr` does not
   exist in Rust's standard library (never has). Unresolved import.
4. **`src-tauri/Cargo.toml`** declares exactly 4 dependencies — `tauri`,
   `tauri-plugin-shell`, `serde`, `serde_json` — while the code uses
   `tokio` (`lib.rs`, throughout), `chrono` (`behavior/monitor.rs` and
   others), and `lru` (`mac/enforcement.rs`'s `LruCache`), none declared.
   The manifest was never kept in sync with 15.5k lines of code written
   against it.
5. **Environment, not code**: `cargo check` fails before reaching any
   Rust-level error — `gdk-3.0` (GTK dev headers) isn't installed on this
   machine, blocking Tauri's Linux (GTK/webkit2gtk) backend from building
   at all. A local system-package gap, unrelated to the code itself.

### 2.3 The dead-code finding: two command surfaces, only the thinner one wired

`src-tauri/src/ui/commands.rs` defines a real, coherent 9-command Tauri
surface (`get_system_status`, `update_system_configuration`,
`analyze_package`, `install_package`, `get_security_alerts`,
`acknowledge_alert`, `get_system_metrics`, plus init) via its own
`register_commands()`. But `lib.rs::run()` never calls it — it builds its
own separate, thinner 3-command `generate_handler!` list instead
(`check_initialization_status`, `complete_initial_setup`,
`initialize_security` — the last of which calls into the broken
`SecuritySystem` from §2.2). The richer, more useful command surface is
complete and unreachable.

### 2.4 Per-module maturity — real signal, not a guess

Scanned every file for real OS-interaction signals (`File::open`,
`read_to_string`, `fs::read`/`write`, literal `/proc`/`/sys` path
strings) and explicit incompleteness markers (`todo!()`/`unimplemented!()`).
This is the honest maturity picture, by subsystem:

| Subsystem | Files (lines) | Real I/O hits | `todo!()` count | Read |
|---|---|---|---|---|
| **kernel/** (secure boot, UEFI) | uefi.rs (992), secure_boot.rs (545), memory.rs (661), syscalls.rs (641), module_verifier.rs (342), runtime_integrity.rs (299) | uefi.rs: **27**, secure_boot.rs: **16**, memory.rs: 3, module_verifier.rs: 1 | 0 across all kernel/ files | **The most real subsystem in the codebase.** Genuine file/proc interaction at real scale. |
| **sandbox/** (resources, network, filesystem, container) | resources.rs (417), network.rs (381), filesystem.rs (434), container.rs (188) | resources.rs: 12, filesystem.rs: 2, network.rs: **0** | 0 | Well-designed skeletons (real cgroup paths modeled, a sensible `NetworkController`/`TrafficMonitor` shape) but largely unimplemented bodies — confirmed by reading `network.rs` directly: `create_virtual_interface()`, `TrafficMonitor::start()` are called but not shown to do real netlink/socket work in what's readable, and the file won't even compile (§2.2 #3). |
| **package/** (analyzers, manager, monitor, installers) | analyzers.rs (748), manager.rs (385), monitor.rs (232), installers.rs (151) | 0 across all | analyzers.rs: **13**, manager.rs: **11**, monitor.rs: 2 | Largest non-kernel file (748 lines) is mostly scaffolding — 13 `todo!()` panics confirmed at exact line numbers (351-423), almost certainly the per-package-format analysis methods (deb/rpm/AppImage/flatpak). Calling any of them today panics, doesn't error gracefully. |
| **mac/** (policy, rules, enforcement, labels + audit/*) | 8 files, 209-369 lines each | 1 total (mac/mod.rs) | 0 | Real, coherent in-memory logic (an `MACEnforcer` with an `AccessCache`/`LruCache`, real `AccessAuditor` audit subsystem with anomaly/retention/storage/patterns submodules) — but **zero actual kernel-level enforcement mechanism** (no LSM, no eBPF, no seccomp hookup visible). This is a real, well-structured *policy engine*, not yet a real *enforcer*. |
| **logging/** (logger, aggregator, analyzer, rotation, storage) | 5 files, 152-556 lines | rotation.rs: 4, others: 0 | 0 | Structurally large (556-line storage.rs) but almost entirely in-memory/simulated — no confirmed real log-file I/O beyond rotation.rs. |
| **integrity/** (monitor, scanner, verifier, baseline) | 4 files, 50-119 lines | scanner.rs: 1 | 0 | The README's headline "System Integrity Monitoring" feature is the **smallest, least-implemented subsystem** — 4 files totaling 307 lines, almost no real file-hash/scan logic evidenced. |
| **behavior/** (monitor, analyzer, patterns, events, system) | 5 files, 105-264 lines | system.rs: 0 (but 4 `todo!()`) | system.rs: 4 | Real event-processing plumbing (`BehaviorMonitor` with a genuine `tokio::spawn` event loop, `mpsc` channels, pattern matching — read directly, this is actually-wired async code, not just structs) — but the system-level event *source* (`system.rs`, presumably where real OS events would originate) is 4× `todo!()`. |
| **ai/** (engine, models, config) | 3 files, 217-344 lines | 0 | engine.rs: **4** | Real structure (`ModelManager`, `PredictionCache`, `TrainingState`) — but `engine.rs`'s core methods panic via `todo!()` at lines 270/275/283/292 (almost certainly predict/train/load/save). No LLM/ML crate confirmed in the bare Cargo.toml either — likely the same missing-dependency problem as everything else. |

**Overall honest read**: this is a real, ambitiously-scoped project with
genuine architectural thought throughout, but only the kernel/UEFI/
secure-boot subsystem has substantial real OS-level implementation. Nearly
everything else — sandboxing, MAC enforcement, package analysis, AI
prediction, integrity monitoring, logging — is well-designed Rust
*scaffolding*: real types, real async plumbing, sensible module
boundaries, but the actual "reach into the OS and do the thing" step is
either a `todo!()` panic or simply absent. This is a materially different
picture from "not started" (it clearly isn't) and from "production-ready"
(it clearly isn't either) — it's a serious in-progress skeleton, further
along in kernel/ than anywhere else.

---

## 3. CereBrix — full review

> **Superseded update (2026-09-29, same day)**: §3.1's original finding
> below describes the directory as it existed at review time — since
> renamed to `~/Projects/CereBrixx` (double-x) and now legacy. The
> operator then added a fresh, real, git-cloned project at
> `~/Projects/Cerebrix` (proper capitalization). §3.4 covers the real
> project; §3.1-3.3 are kept as an accurate record of what was true at
> the time, not deleted, since the old RatModel.py is still real and
> still in active use (§4.2, `rat_model_generate`).

### 3.1 What's actually there (original finding, now the legacy folder)

The entire `~/Projects/CereBrixx` directory contains **one file**:
`RatModel.py` (19KB, no README, no other code, no supporting assets).

Read in full. It is a Blender Python (`bpy`/`bmesh`) script:
`AnatomicalRatModel`, a class that procedurally generates a realistic 3D
rat model — a real armature/skeleton (spine, neck, head, 4 legs with
upper/lower/foot bones, an 8-segment tail), a muscled body mesh with
shoulder/hip bulges, a detailed head (eyes with real shader materials,
ears, 3-row whisker arrays via bezier curves), procedural fur material
(noise-texture-driven color variation + bump mapping), tail scale
material (Voronoi-pattern), and 4 paws with anatomically-correct toe
counts (4 front, 5 hind) parented to the armature's foot bones. Real
measurements are hardcoded in Blender units (e.g. `TOTAL_LENGTH = 2.4`
= 24cm, matching a real average lab-rat body length). `main()` sets
Cycles/GPU rendering and calls `assemble_model()`.

**Honest finding, stated directly**: there is **no EEG, neural-signal,
brain-data, or biosignal-processing code anywhere in this file or
directory**. This is a 3D anatomical asset generator, full stop — not a
data pipeline, not a signal processor, not a modality implementation of
any kind.

### 3.2 The gap between what exists and what the operator described

The operator asked to "capture the EEG modality we will or ZCode will
activate" alongside CereBrix — implying an expected EEG connection.
Stating the honest gap rather than papering over it: **CereBrix as it
exists today has no relationship to EEG signal data.** What it plausibly
IS — labeled here explicitly as inference, not something the code
confirms — is a **physical/anatomical reference model**, the kind
commonly needed in real rodent-EEG research for planning electrode
placement, visualizing a montage, or producing a digital-twin reference
for a physical rat subject. That would make it a genuine, real piece of
the eventual "rat EEG research" picture — just the **3D/anatomical**
piece, not the **signal-capture** piece. The actual EEG modality (108) —
confirmed elsewhere in this repo's own inventory (MODALITY_EXPANSION_
GUIDE.md §5.1 item 27) as "compiled, unrevived" — is a wholly separate,
not-yet-started piece of work.

### 3.3 Why this maps cleanly onto Ozone-Studio's existing physical-bridge pattern

CereBrix being a Blender script is not a dead end — it's a near-perfect
fit for the **already-live** Blender MCP bridge (bridge 203, proven this
session against a real scene, container 40232) documented in
MODALITY_EXPANSION_GUIDE.md §3. That bridge already does real scene
creation/mutation via `blender_get_scene`/`create_object`-style calls.
`RatModel.py`'s `assemble_model()` is exactly the shape of script the
physical bridge is meant to execute and then import the resulting mesh
from — no new architecture required, just a new script target.

### 3.4 The real Cerebrix (`~/Projects/Cerebrix`) — full review, current

A genuine, substantially-real **BCI AI Controller API** — git-cloned,
real README, LICENSE, real git history (15 commits: feature extractors →
model architectures → intent detection/calibration → docs). Confirms the
operator's framing directly: this DOES cover EEG monitoring, and the
`Addons/AnimalInterface/{Cat,Mouse,Rat}` structure DOES generalize to
animals — both true, unlike §3.1's now-superseded folder.

**Real architecture, read directly (not from the README alone)**:
- **Windowing**: `util/window_processor.py` (220 lines, 0 stub signals) —
  real numpy dataclass-based sliding-window logic, exactly matching the
  README's documented 200ms/5-frame-at-25Hz behavior with temporal
  zero-padding at sequence edges.
- **Feature extraction**: handcrafted (band powers, Hjorth parameters,
  spatial connectivity — `temporal_spectral_extractor.py`, 228 lines,
  real scipy signal processing) + unsupervised (autoencoder, Deep
  Embedded Clustering, recurrent autoencoder, SOM).
- **Model architectures**: 5 real families, each in raw+features-only
  variants — CNN-LSTM, GNN, Hierarchical Attention Network, Transformer
  (`raw_features_transformer.py` uses a real custom `TransformerDecoder`
  from `decision_layers/`).
- **Orchestration**: `EEGAnalysis.py` (323 lines, 0 stub signals) ties
  every model family + `IntegratedTrainer` + `WindowProcessor` together,
  uses real `mlflow` experiment tracking.
- **Live streaming GUI**: `Cerebrix.py` (898 lines, 1 stub signal) — a
  real PyQt5 desktop app streaming EEG via **pylsl** (Lab Streaming
  Layer — the actual real, industry-standard protocol most EEG/biosignal
  hardware, including OpenBCI and Muse, speaks), live 3D visualization
  (matplotlib/plotly).
- **Data**: `data_manager.py` — real `mmap` + `portalocker` file locking,
  not a toy in-memory stub.

**Real dependencies confirmed via direct import statements** (no
requirements.txt/setup.py exists — same undeclared-manifest pattern as
VerdadX, though lower-stakes for Python): `numpy` (confirmed installed
on this host), `scipy`, `tensorflow`, `PyQt5`, `pylsl`, `plotly`,
`matplotlib`, `mlflow`, `portalocker` (confirmed NOT installed — the full
ML pipeline needs a venv build before it can run live, same shape as
visual-mcp's earlier YOLO/Depth-Anything venv).

**AnimalInterface honesty check**: `Addons/AnimalInterface/{Cat,Mouse,
Rat}/` each contain **only a README** in the new repo — no actual
Blender integration code yet for any of the three animals. The one real,
working 3D asset remains the legacy `CereBrixx/RatModel.py` (§3.1) — that
is what §4.2's `rat_model_generate` tool wraps.

---

## 4. MCP strip & integration plan

> **Status update (2026-09-29, same day)**: the operator approved moving
> from plan to build. `tools/security-mcp/` and `tools/cerebrix-mcp/` are
> now real, running, registered servers — §4.1/§4.2 below are updated to
> show what's actually live vs. still planned, not left as a stale plan.

### 4.1 VerdadX → a `security-mcp` (proposed name)

**Prerequisite repair** (the 5 items in §2.2) is not optional — nothing
downstream can be verified live until at least a `cargo check` clean
pass exists. Two honest paths, tradeoffs stated plainly (this is the
fork the operator already flagged wanting a real decision on):

- **Path A — fresh headless core.** New standalone Rust crate that does
  NOT depend on `tauri`/`gdk` at all (sidesteps §2.2 #5 entirely), reuses
  VerdadX's real kernel/uefi.rs + kernel/secure_boot.rs logic (the
  genuinely strong subsystem) directly as library code, and implements
  real process/network/resource monitoring using proper crates
  (`procfs`/`nix`/`sysinfo` for process+resource, real `/proc`/`/sys`
  reads, `netlink`-based connection listing) rather than the currently-
  unimplemented `sandbox/network.rs`/`resources.rs` bodies. Fastest path
  to a genuinely working monitor; leaves MAC/behavior/package/AI/
  integrity as later phases once each is worth the same treatment.
- **Path B — repair VerdadX in place.** Fix the 5 real bugs (define
  `SecuritySystem`, fix the `package_analyzer` typo, remove/replace the
  invalid `MacAddr` import with a real crate type, fill in the missing
  Cargo.toml dependencies, install `gdk-3.0` locally), get a clean
  `cargo check`, THEN wrap the resulting (still mostly-scaffolded per
  §2.4) `SecuritySystem` as the MCP's backend. Slower to a working
  monitor (most subsystems still need their `todo!()`s filled regardless
  of whether the crate compiles), but keeps VerdadX's fuller original
  vision — MAC enforcement, package analysis, AI prediction, secure boot
  — as one coherent evolving system rather than a parallel rewrite.

**Proposed tool surface** (module → MCP tool → real readiness today),
independent of which path is chosen:

| Proposed tool | Source module | Readiness today |
|---|---|---|
| `secure_boot_status` | kernel/secure_boot.rs, kernel/uefi.rs | **Real logic exists** (27+16 real I/O hits) — closest to a genuine near-term tool |
| `resource_usage` (the "power-hungry / over-consuming" ask) | sandbox/resources.rs | Design real (cgroup controllers), body mostly unimplemented — needs real work either path |
| `network_activity` (the "better Wireshark" ask) | sandbox/network.rs | Design present, **zero real I/O, won't compile** — needs the most new work of any tool |
| `mac_policy_status` | mac/*, mac/audit/* | Real in-memory policy engine, no real kernel enforcement — tool would report POLICY state honestly, not real enforcement, until that gap is closed |
| `package_analyze` | package/analyzers.rs | 13 `todo!()` panics in the exact methods this tool would call — not safe to wrap yet without guarding each panic path |
| `integrity_scan` | integrity/* | Smallest, least-implemented subsystem — lowest near-term priority |
| `behavior_alerts` | behavior/monitor.rs, alerts/manager.rs | Real async event-loop plumbing exists; needs a real event *source* (behavior/system.rs's `todo!()`s) to have anything to report |
| `ai_threat_predict` | ai/engine.rs | Real structure, core methods `todo!()` — lowest near-term priority alongside integrity |

### 4.2 STATUS: built and live (2026-09-29), with one operator-corrected scope fix

**Operator correction, applied**: the rat-model tool is NOT part of
Cerebrix (EEG) — it's a 3D-modality (109) tool, and its real purpose is
narrower than "anatomical reference": it's for **EEG-cap costume
simulation** (fitting/designing a physical electrode cap against a
measured 3D rat body). Originally built inside `tools/cerebrix-mcp/`;
moved to a new `tools/threed-mcp/` once the boundary was corrected.

- **`tools/cerebrix-mcp/`** (:3245) — EEG-only now. `eeg_window` wraps
  the real, unmodified `WindowProcessor.create_sliding_windows()` from
  the real Cerebrix repo (§3.4) — numpy-only, live-verified through the
  full gated `/mcp/call` stack.
- **`tools/threed-mcp/`** (:3250) — `eeg_cap_costume_generate` wraps the
  real, unmodified `AnatomicalRatModel.assemble_model()` from the legacy
  `CereBrixx/RatModel.py` via headless Blender, exporting glTF. **Honest
  status, found and isolated, not hidden**: the full sequence hangs
  headless somewhere between body- and head-completion (skeleton+body
  alone, and eyes/ears alone, both complete correctly in isolation — the
  full chain doesn't within 30s). Wrapped with a hard timeout; fails
  cleanly rather than hanging the server. Root cause not chased further
  — a real, scoped limitation, not a guess.
- EEG modality (108) activation itself remains separate, not started —
  per the existing Wave-4 inventory in MODALITY_EXPANSION_GUIDE.md.

### 4.3 Fruit fly connectome — `tools/connectome-mcp/` (:3255), BUILT and LIVE

New capability, operator directive same day: "the NN of the fruit fly,
download it and plug it in, study it, monitor it." Real data — the
2024 Nature-published FlyWire/Codex FAFB whole-brain Drosophila
connectome, 139,248 real annotated neurons (real 3D position, cell
type/class, neurotransmitter + confidence, hemilineage, side), sourced
from the real public `github.com/flyconnectome/flywire_annotations`
repo after confirming the official Codex download app requires
authentication this session doesn't have. Two tools: `fly_neuron_query`
(filtered real records) and `fly_connectome_stats` (real aggregates,
computed live, not cached — e.g. optic-system dominance at 77,541/
139,248 neurons, matching known Drosophila neuroanatomy). **Honest
boundary**: neuron catalog only — real synapse-level connectivity
(3.7M+ real connections, the actual wiring) needs FlyWire/Codex
credentials not available in this session; not fabricated to fill the
gap. Both tools live-verified through the full gated `/mcp/call` stack.

---

## 5. Open questions for the operator (not decided here)

1. **VerdadX path**: fresh headless core (A) vs. repair-in-place (B) —
   §4.1 lays out the real tradeoff; no default chosen.
2. **VerdadX build order**: §4.1's table orders tools by real readiness
   (secure-boot first, network/AI/integrity last) — confirm that's the
   right priority, or reorder to match what's actually wanted first (the
   operator's own words leaned hardest toward network/resource/power
   monitoring, which is honestly the LEAST-implemented part today).
3. **CereBrix**: confirmed as a 3D/anatomical asset, not an EEG asset —
   is `rat_model_generate` via the Blender bridge still wanted as
   described, or was a different CereBrix (elsewhere, or not yet
   created) actually intended?
