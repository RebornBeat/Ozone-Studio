// android-mcp selftest — REAL adb (platform-tools), real server process. No device is attached in
// CI-less dev, so device-facing positives are reported as SKIP (never faked); policy, validation and
// preflight honesty are exercised for real. With a device/emulator attached, pass its serial as argv[2]
// to also run snapshot/term/screenshot checks.
import { spawn } from "node:child_process";
const [, , SERIAL] = process.argv;
const PORT = 3292;
const env = { ...process.env, OZONE_ANDROID_PORT: String(PORT), OZONE_ANDROID_DATA_DIR: "/tmp/oz-android-test" };
const locked = spawn("node", [new URL("./server.mjs", import.meta.url).pathname], { env: { ...env, OZONE_ANDROID_PORT: String(PORT + 1) }, stdio: "ignore" });
const open = spawn("node", [new URL("./server.mjs", import.meta.url).pathname], { env: { ...env, OZONE_ANDROID_SHELL_ALLOW: "getprop,ls,pm,settings,rm,cd,id", OZONE_ANDROID_TOKEN: "t0k" }, stdio: "ignore" });
await new Promise((r) => setTimeout(r, 900));
let pass = 0, fail = 0, skip = 0;
const call = async (port, tool, input = {}, tok = "t0k") => (await fetch(`http://127.0.0.1:${port}/call`, { method: "POST", headers: { "x-ozone-android-token": tok }, body: JSON.stringify({ tool, input }) })).json();
const ok = (n, c, x = "") => { c ? pass++ : fail++; console.log(`${c ? "PASS" : "FAIL"}  ${n}${x ? "  — " + x : ""}`); };
const sk = (n, why) => { skip++; console.log(`SKIP  ${n}  — ${why}`); };
const S = SERIAL ?? "emulator-5554";
let r = await fetch(`http://127.0.0.1:${PORT}/call`, { method: "POST", body: JSON.stringify({ tool: "android_devices" }) }).then((x) => x.status);
ok("token required (401 without header)", r === 401);
r = await call(PORT + 1, "android_shell", { serial: S, command: "getprop", confirm: true }, "");
ok("shell LOCKED by default", !r.success && /LOCKED/.test(r.error), r.error);
r = await call(PORT, "android_shell", { serial: S, command: "getprop" });
ok("shell needs confirm:true", !r.success && /confirm/.test(r.error));
for (const [label, c] of [["su", "su -c id"], ["rm -rf", "rm -rf /sdcard"], ["pm uninstall", "pm uninstall com.x"], ["settings put", "settings put global x 1"], ["operator ;", "ls; id"], ["operator |", "ls | id"], ["subshell $()", "ls $(id)"], ["off-allowlist", "cat /etc/hosts"]]) {
  r = await call(PORT, "android_shell", { serial: S, command: c, confirm: true });
  ok(`refused: ${label}`, !r.success && /refused|rejected|not in OZONE/.test(r.error), r.error?.slice(0, 60));
}
r = await call(PORT, "android_forward", { serial: S, local: 18022, remote: 22 });
ok("forward: non-whitelisted remote port refused", !r.success);
r = await call(PORT, "android_emulator_start", { avd: "x" });
ok("emulator control LOCKED by default", !r.success && /LOCKED/.test(r.error));
r = await call(PORT, "android_emulator_stop", { serial: "R58M123456" });
ok("emulator_stop LOCKED (and would refuse a physical serial)", !r.success);
r = await call(PORT, "android_term_exec", { term: "nope", command: "ls", confirm: true });
ok("term_exec unknown session refused", !r.success);
r = await call(PORT, "android_preflight");
ok("preflight reports honestly", r.success && typeof r.output.kvm.present === "boolean" && "ready_for_device" in r.output, `adb=${r.output?.adb?.version?.slice(0, 40)} kvm=${JSON.stringify(r.output?.kvm)} emu=${r.output?.emulator_binary}`);
r = await call(PORT, "android_devices");
ok("android_devices via real adb", r.success && Array.isArray(r.output.devices), `${r.output?.devices?.length} device(s)`);
if (r.output?.devices?.some((d) => d.serial === S && d.state === "device")) {
  r = await call(PORT, "android_device_info", { serial: S }); ok("device_info", r.success, JSON.stringify(r.output).slice(0, 80));
  r = await call(PORT, "android_ui_snapshot", { serial: S }); ok("ui_snapshot returns elements", r.success && r.output.count >= 0, `${r.output?.count} elements`);
  r = await call(PORT, "android_term_open", { serial: S }); const t = r.output?.term; ok("term_open", r.success);
  r = await call(PORT, "android_term_exec", { term: t, command: "cd /sdcard", confirm: true }); ok("term cd tracks cwd", r.success && r.output.cwd === "/sdcard");
  r = await call(PORT, "android_term_exec", { term: t, command: "ls", confirm: true }); ok("term ls runs in cwd", r.success && r.output.exit_code === 0);
} else sk("device-facing positives (info, ui_snapshot, term_*)", `no device '${S}' attached — needs an emulator (KVM+SDK) or a USB-debugging phone`);
locked.kill(); open.kill();
console.log(`\n${pass} passed, ${fail} failed, ${skip} skipped`); process.exit(fail ? 1 : 0);
