# Mobile / Android in Ozone-Studio — state, security review, and the terminal-like path (2026-10-29, cc)

Operator ask: review the security MCP and logs (local + EU); identify the mobile MCP; how to work with mobile "like Linux"
(terminal sense, at least Android); review ZCode's Android emulator and bring an MCP of it into Ozone; enrich security,
terminal and mobile MCPs; capture and test.

## 1. What exists (verified by reading + running)
| Piece | What it is |
|---|---|
| `tools/android-mcp` (:3277) | Ozone's adb MCP. Was 7 tools, `android_shell` unrestricted behind `confirm:true`. Now 18 (see README). |
| `tools/ozone-mobile` | Expo/React-Native companion app (pairing/QR). Not an MCP; the pairing flow it uses is the same one browser sessions use. |
| ZCode `android-emulator` plugin | ZCode-PRIVATE plugin (AppImage, 22 tools: preflight, discover/create/build app, AVD list/create/start/stop, install/launch/terminate, open_url, screenshot, logs, ui_describe/resolve/tap/swipe/type). Needs an Android SDK + JDK + Gradle. It is not an Ozone tool and bypasses Ozone's gate. |
| Host supervisor | spawns android-mcp only when a device is present (`mcp_supervisor.rs`). |

Decision: do NOT re-wrap ZCode's plugin; absorb its *capability list* into `android-mcp` behind Ozone's door. Done:
preflight, AVD list/start/stop, ui_snapshot/tap_element (perceive->act like `browser_snapshot`), screenshot, logs, install.
Not absorbed (needs SDK/Gradle, no value without an emulator): create_app/build_app/discover_project.

## 2. Terminal sense on Android (no root)
1. **Stock phone, USB/Wireless debugging approved** -> `android_term_*`: a session with tracked cwd, each command through the
   same allowlist as `android_shell` (adb shell is a `shell`-uid sandbox: no root, no other app data).
2. **Real shell feel:** install **Termux** (F-Droid), `pkg install openssh && sshd` (port 8022), then
   `android_forward {local, remote:8022}` and `ssh -p 8022 localhost` from a terminal-mcp role. Gives bash, git, python, node
   on the phone. `android_termux_status` checks it is installed. This is the only route to a Linux userland without root.
3. **Rooted/emulator (userdebug):** `adb root` gives uid 0; `android_device_info.root` reports it, never assumed.
4. **iOS:** no equivalent (no adb); only the Expo app + pairing. Out of scope.
Nothing runs by itself: the Personal Assistant may only *propose* an action (see MASTER_REMAINING_WORK_GUIDE W2).

## 3. Emulator — RUN and verified (2026-10-29)
Provisioned user-space, no sudo, by `tools/android-mcp/provision_sdk.sh`: JDK 17 (`~/Android/jdk17`), SDK at `~/Android/Sdk`
(cmdline-tools, emulator, platform-tools 37.0.1, `system-images;android-30;default;x86_64`), AVD `ozone_api30` (1.5 GB RAM,
swiftshader GPU, 2 GB data). 4.8 GB on disk. `/dev/kvm` is usable via ACL -> **booted headless in ~65 s**. Started and driven
ONLY through `/mcp/call` (gate not blocked, metered). Acceptance: `selftest.mjs emulator-5554` = **22/22, 0 skipped**;
live script: device_info (Android 11 / API 30 / x86_64 / uid 2000, root:false), ui_snapshot, tap-by-label ("Messaging"),
re-snapshot shows the Messaging app, term cd/ls/getprop in a tracked-cwd session, refusals (`su`, `rm -rf`, pipes, forward :22),
screenshot, logcat, termux_status (not installed), forward 8022. Evidence: `docs/evidence/android-mcp/`.
Note: `sdkmanager --licenses` was accepted (Google SDK license) because the operator directed running an emulator.
Cost: with the emulator up, available RAM fell to ~1.5-2.2 GB on this 7.6 GB box (ZCode alone ~4-5 GB) — stop it when idle
(`android_emulator_stop`).

## 3b. Relationship to ZCode's emulator plugin — "are we creating our own?"
Two different layers, deliberately not duplicates:
- **ZCode's `android-emulator` plugin** (Apache-2.0, stdio MCP, 22 tools): a *developer workflow* for ZCode — create/build a
  Compose app with Gradle, run it, screenshot, UI-describe. It had no SDK/JDK/AVD on this machine either (it only discovers
  one at `ANDROID_PLUGIN_SDK_PATH`/`ANDROID_HOME`/`~/Android/Sdk`). It does not pass through Ozone's gate, ledger or jurisdiction.
- **Ozone `android-mcp`**: the *governed* layer — locked-by-default policy, metering, gate, ripple into the graph — so the
  orchestrator / Personal Assistant / any agent can use a phone or emulator under Ozone's rules.
They now SHARE one SDK (`~/Android/Sdk`, same default path both discover), so provisioning is done once. How ZCode benefits:
its plugin can now build+install apps against a real AVD with zero extra setup (point `default_avd` at `ozone_api30`).
How Ozone benefits: a test device for the mobile companion app (`tools/ozone-mobile`) and for agent-driven UI testing with
screenshots into the image modality. Not absorbed from ZCode (needs Gradle, only matters when building apps): create_app,
build_app, discover_project, ui_describe semantic resolver. Candidate next step if wanted: a thin Ozone Pattern-B bridge that
calls ZCode's stdio server for build/run and keeps Ozone's door in front.

## 4. Security review — findings and fixes
| # | Finding | Evidence | Fix |
|---|---|---|---|
| 1 | Local `firewall_status` failed every 15 min (unprivileged), so the sensor tick raised the same finding repeatedly and the I-Loop reflected on its own echo | `host.log` sensor-tick lines 12:51, 13:36, 02:40, 03:25; I-Loop reflections quoting it | `firewall_status` now answers `success:true, inspectable:false` + service state (UNKNOWN, never "no rules"); real failures still fail. Verified through `/mcp/call`. |
| 2 | **EU is healthy.** zcode's "0 sensor events on EU / MCP registration gap" was wrong | EU: registry has all 4 tools; 133 `sensor-tick` events in `shared_context/`, all `success:true`, newest minutes old. The tick logs only failures, so log-grepping showed nothing. EU unit has `CAP_NET_ADMIN`, `firewall_status` returns real ufw rules | none needed |
| 3 | terminal-mcp first-token allowlist is bypassable by any allowlisted interpreter/runner | `node -e`, `python3 -c`, `find -exec/-delete`, `git -c`/`config`, `npm exec`, `systemctl restart` all passed | per-command argument denies; 18/18 selftest incl. operator/newline/`nodemon` cases. Residual: `node script.js` / `python3 file.py` still run repo files (by design). |
| 4 | android_shell ran any command | code comment "nothing here filters commands" | locked-by-default allowlist + deny verbs + token; 17/17 selftest |
| 5 | android-mcp returned HTTP 400 for refusals; host drops non-2xx bodies so the agent never saw why | observed "tool returned HTTP 400" | 200 + `success:false` + reason |
| 6 | New read-only capability: `exposure_report` (loopback vs external listeners, risky ports) and `security_status` | local: 27 listeners, 8 external, 0 risky | added |
Other log facts: local host healthy, supervisor spawned security/p2p-bridge; I-Loop reflections still dominated by the firewall
echo until the next sensor cycle proves #1 (verify at the next 15-min tick, do not assume).

## 5. Open
- Deploy `security-mcp` (exposure_report) and the terminal-mcp deny rules to EU (`/opt/ozone-tools/...`, then restart units) — small, not yet done.
- Register terminal-mcp changes need no re-registration (same tools); new android/security tools ARE registered locally (18+2).
- Phone pairing test with a real device; Termux sshd path test (install Termux in the emulator or a phone); `ozone-mobile` APK on the AVD.
- Android tools not yet in `capability_ids` planning (same gap as browser tools).
