# android-mcp — Ozone's governed Android control (Pattern A)

adb-based, loopback-only (`:3277`), invoked only through `/mcp/call` (metered, gated, rippled). Full context:
`docs/MOBILE_ANDROID_GUIDE.md`.

**Locked by default.** `android_shell` / `android_term_exec` refuse everything until `OZONE_ANDROID_SHELL_ALLOW`
(comma list of device commands; `*` = explicit opt-in) is set. One command per call (`; | & \` $ < >` and newlines refused);
`su reboot dd rm -rf pm uninstall settings put am force-stop …` are denied even if allowlisted. Emulator control needs
`OZONE_ANDROID_ALLOW_EMULATOR=1` and a usable `/dev/kvm`; `android_emulator_stop` only accepts `emulator-NNNN`.
Optional `OZONE_ANDROID_TOKEN` (header `x-ozone-android-token`). All refusals are HTTP 200 `success:false` + reason
(the host hides the body of non-2xx answers).

Tools: `android_preflight devices device_info avd_list emulator_start emulator_stop shell screenshot input install logcat
ui_snapshot ui_tap_element term_open term_exec term_close forward termux_status`.

Launch (operator shell / supervisor; agent sessions reap children): `target/release/launch-ops-mcps.sh`
(adb at `~/Android/platform-tools`). Register: `node tools/register-browser-tools.mjs android`.
Test: `PATH=$HOME/Android/platform-tools:$PATH node tools/android-mcp/selftest.mjs [serial]` — real adb; device-facing
checks are SKIPPED (never faked) when no device is attached.

Provision a real SDK + headless AVD (user-space, no sudo): `tools/android-mcp/provision_sdk.sh` (shared with ZCode's android-emulator plugin).
