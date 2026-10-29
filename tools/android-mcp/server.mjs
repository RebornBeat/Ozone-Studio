// Ozone-Studio Android MCP — real control of a paired Android device via adb.
//
// Access model (stock Android, no root needed):
//   - Developer options + USB debugging, or Wireless debugging (Android 11+),
//     with the RSA prompt approved on the phone. That is the only path to a
//     device shell from a computer; no installed app can grant it to itself.
//   - Root is detected at runtime (uid 0) and reported, never assumed.
//   - Device state must be "device". "unauthorized" means the phone prompt
//     has not been approved yet.
//
// Tools (POST /call {tool, input}):
//   android_devices {}                                  → attached devices + state
//   android_device_info {serial}                        → model, Android release, API level, security patch, abi, uid, root
//   android_shell {serial, command, confirm:true, timeout_s?}
//                                                        → device shell command, stdout, real exit code
//   android_screenshot {serial}                         → PNG via screencap, saved under data/screenshots
//   android_input {serial, kind:"tap"|"swipe"|"text"|"keyevent", ...}
//   android_install {serial, apk}                       → adb install -r of data/apks/<apk>
//   android_logcat {serial, lines?}                     → last N logcat lines
//   android_preflight {}                                → adb/SDK/KVM/policy readiness, honest
//   android_avd_list / android_emulator_start|stop       → headless emulator lifecycle (OZONE_ANDROID_ALLOW_EMULATOR=1, needs usable KVM)
//   android_ui_snapshot / android_ui_tap_element        → perceive (uiautomator) -> act loop, labels untrusted
//   android_term_open|exec|close                        → terminal-like session with tracked cwd
//   android_forward / android_termux_status             → reach a human-installed Termux sshd (real shell, no root)
//
// android_shell and term_exec are LOCKED unless OZONE_ANDROID_SHELL_ALLOW is set (comma list or *).
// Optional OZONE_ANDROID_TOKEN adds header x-ozone-android-token for direct callers.
//
// Notes:
//   - android_shell runs an arbitrary command on the device by design. It
//     requires confirm:true so an agent cannot run it by accident. The host's
//     gate and per-device pairing are the controls; nothing here filters commands.
//   - Host-side, adb is always invoked with an argument array (no shell), so
//     serials and file names cannot inject host commands.
//   - Requires adb (Android platform-tools) on PATH, or OZONE_ADB, or
//     ANDROID_HOME/platform-tools/adb. Missing adb is reported, not faked.
//
// Env: OZONE_ANDROID_PORT (default 3277), OZONE_ANDROID_DATA_DIR, OZONE_ADB.

import { createServer } from "node:http";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { mkdir, writeFile, realpath } from "node:fs/promises";
import { existsSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const execFileP = promisify(execFile);
const HERE = path.dirname(fileURLToPath(import.meta.url));
const PORT = Number(process.env.OZONE_ANDROID_PORT ?? 3277);
const DATA_DIR = path.resolve(process.env.OZONE_ANDROID_DATA_DIR ?? path.join(HERE, "data"));
const SCREENSHOT_DIR = path.join(DATA_DIR, "screenshots");
const APK_DIR = path.join(DATA_DIR, "apks");
const MAX_BUFFER = 16 * 1024 * 1024;
const SERIAL_RE = /^[A-Za-z0-9._:-]{1,80}$/;
const PNG_SIG = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

class InputError extends Error {}

// ---- Shell policy (locked by default, same doctrine as terminal-mcp) ----------
// The old android_shell ran ANY device command given confirm:true — "nothing here
// filters commands". Now the first token must be in OZONE_ANDROID_SHELL_ALLOW
// (comma list; "*" is an explicit opt-in to anything). Unset = LOCKED. Shell
// operators are refused (one command per call) and a deny list covers the
// state-destroying verbs even for an allowlisted tool.
const SHELL_ALLOW = (process.env.OZONE_ANDROID_SHELL_ALLOW ?? "").split(",").map((x) => x.trim()).filter(Boolean);
const ALLOW_EMULATOR = process.env.OZONE_ANDROID_ALLOW_EMULATOR === "1";
const TOKEN_SECRET = process.env.OZONE_ANDROID_TOKEN ?? "";
const FORBIDDEN = /[;|&`$><\n\r\0]/;
const SHELL_DENY = {
  su: () => true, reboot: () => true, recovery: () => true, dd: () => true, mkfs: () => true, rm: (a) => a.some((x) => /^-[a-z]*[rf]/.test(x)),
  pm: (a) => a.some((x) => /^(uninstall|clear|disable|disable-user|hide|suspend)$/.test(x)),
  settings: (a) => a[0] === "put" || a[0] === "delete",
  am: (a) => a.some((x) => /^(force-stop|kill|kill-all|crash)$/.test(x)),
  svc: () => true, wipe: () => true, setprop: () => true, chmod: () => true, chown: () => true,
};
function shellPolicy(command) {
  if (!SHELL_ALLOW.length) throw new InputError("android_shell is LOCKED: OZONE_ANDROID_SHELL_ALLOW is not configured (comma list of device commands, or * to opt in to all)");
  if (FORBIDDEN.test(command)) throw new InputError("shell operators (; | & ` $ < > and newlines) are rejected — one command per call");
  const parts = command.trim().split(/\s+/);
  const first = parts[0];
  if (!SHELL_ALLOW.includes("*") && !SHELL_ALLOW.includes(first)) throw new InputError(`command '${first}' is not in OZONE_ANDROID_SHELL_ALLOW (${SHELL_ALLOW.join(", ")})`);
  const deny = SHELL_DENY[first];
  if (deny && deny(parts.slice(1))) throw new InputError(`'${command.slice(0, 40)}' is refused — destructive/state-changing device verb`);
}

// One SDK shared with ZCode's android-emulator plugin (same ANDROID_HOME / default location),
// so adb, the emulator and AVDs are provisioned once (tools/android-mcp/provision_sdk.sh).
const SDK_ROOT = [process.env.ANDROID_HOME, process.env.ANDROID_SDK_ROOT, path.join(os.homedir(), "Android", "Sdk")].find((p) => p && existsSync(p)) ?? null;
const JAVA_HOME = process.env.JAVA_HOME || (existsSync(path.join(os.homedir(), "Android", "jdk17")) ? path.join(os.homedir(), "Android", "jdk17") : undefined);
function findAdb() {
  if (process.env.OZONE_ADB) return process.env.OZONE_ADB;
  for (const root of [SDK_ROOT, path.join(os.homedir(), "Android")]) {
    if (!root) continue;
    const p = path.join(root, "platform-tools", "adb");
    if (existsSync(p)) return p;
  }
  return "adb";
}

async function adb(args, { timeoutMs = 30000, binary = false } = {}) {
  const bin = findAdb();
  try {
    const { stdout } = await execFileP(bin, args, {
      timeout: timeoutMs,
      maxBuffer: MAX_BUFFER,
      encoding: binary ? "buffer" : "utf8",
    });
    return stdout;
  } catch (err) {
    if (err.code === "ENOENT") {
      throw new Error(`adb not found (tried ${bin}). Install Android platform-tools or set OZONE_ADB.`);
    }
    const detail = String(err.stderr ?? err.message ?? "").trim().slice(0, 500);
    throw new Error(`adb ${args.join(" ").slice(0, 80)} failed: ${detail}`);
  }
}

async function requireDevice(serial) {
  if (!SERIAL_RE.test(String(serial ?? ""))) throw new InputError("serial is required (see android_devices)");
  const out = await adb(["devices"]);
  const row = out.split(/\r?\n/).map((l) => l.trim().split(/\s+/)).find((p) => p[0] === serial);
  if (!row) throw new InputError(`device ${serial} is not connected`);
  if (row[1] !== "device") {
    throw new InputError(`device ${serial} state is '${row[1]}' — approve the debugging prompt on the phone`);
  }
}

function intIn(name, value, min, max) {
  const n = Number(value);
  if (!Number.isInteger(n) || n < min || n > max) {
    throw new InputError(`${name} must be an integer in [${min}, ${max}]`);
  }
  return n;
}

const TERMS = new Map();
const tools = {
  async android_devices() {
    const out = await adb(["devices", "-l"]);
    const devices = out
      .split(/\r?\n/)
      .slice(1)
      .map((l) => l.trim())
      .filter(Boolean)
      .map((l) => {
        const [serial, state, ...rest] = l.split(/\s+/);
        const kv = Object.fromEntries(rest.map((p) => p.split(":")).filter((p) => p.length === 2));
        return { serial, state, model: kv.model ?? null, product: kv.product ?? null, transport_id: kv.transport_id ?? null };
      });
    return { devices };
  },

  async android_device_info(input) {
    await requireDevice(input.serial);
    const props = [
      "ro.product.manufacturer",
      "ro.product.model",
      "ro.build.version.release",
      "ro.build.version.sdk",
      "ro.build.version.security_patch",
      "ro.product.cpu.abi",
    ];
    const cmd = [...props.map((p) => `getprop ${p}`), "id -u"].join("; ");
    const lines = (await adb(["-s", input.serial, "shell", cmd])).split(/\r?\n/);
    const uid = (lines[6] ?? "").trim();
    return {
      serial: input.serial,
      manufacturer: lines[0]?.trim() || null,
      model: lines[1]?.trim() || null,
      android_release: lines[2]?.trim() || null,
      api_level: Number(lines[3]) || null,
      security_patch: lines[4]?.trim() || null,
      abi: lines[5]?.trim() || null,
      uid: uid === "" ? null : Number(uid),
      root: uid === "0",
    };
  },

  async android_shell(input) {
    if (input.confirm !== true) {
      throw new InputError("android_shell runs an arbitrary device command; pass confirm:true to run it");
    }
    const command = String(input.command ?? "");
    if (!command.trim() || command.length > 2000) throw new InputError("command must be 1-2000 characters");
    shellPolicy(command); // policy first: cheap, and a refused command never touches the device
    await requireDevice(input.serial);
    const timeoutS = input.timeout_s === undefined ? 30 : intIn("timeout_s", input.timeout_s, 1, 300);
    const wrapped = `${command}\n__ozone_rc=$?; echo __OZONE_EXIT:$__ozone_rc`;
    const out = await adb(["-s", input.serial, "shell", wrapped], { timeoutMs: timeoutS * 1000 });
    const m = out.match(/__OZONE_EXIT:(\d+)\s*$/);
    const stdout = m ? out.slice(0, m.index) : out;
    return { serial: input.serial, stdout, exit_code: m ? Number(m[1]) : null };
  },

  async android_screenshot(input) {
    await requireDevice(input.serial);
    const png = await adb(["-s", input.serial, "exec-out", "screencap", "-p"], { binary: true });
    if (!png.subarray(0, 8).equals(PNG_SIG)) throw new Error("screencap did not return a PNG");
    await mkdir(SCREENSHOT_DIR, { recursive: true });
    const file = path.join(SCREENSHOT_DIR, `${input.serial}-${Date.now()}.png`);
    await writeFile(file, png);
    return { serial: input.serial, path: file, bytes: png.length, width: png.readUInt32BE(16), height: png.readUInt32BE(20) };
  },

  async android_input(input) {
    let args;
    if (input.kind === "tap") {
      args = ["input", "tap", intIn("x", input.x, 0, 10000), intIn("y", input.y, 0, 10000)];
    } else if (input.kind === "swipe") {
      args = [
        "input", "swipe",
        intIn("x", input.x, 0, 10000), intIn("y", input.y, 0, 10000),
        intIn("x2", input.x2, 0, 10000), intIn("y2", input.y2, 0, 10000),
        intIn("duration_ms", input.duration_ms ?? 300, 1, 10000),
      ];
    } else if (input.kind === "text") {
      const escaped = String(input.text ?? "").replace(/ /g, "%s");
      if (!/^[A-Za-z0-9._,@:%-]{1,200}$/.test(escaped)) {
        throw new InputError("text supports letters, digits, spaces and . , @ : - only, up to 200 characters");
      }
      args = ["input", "text", escaped];
    } else if (input.kind === "keyevent") {
      args = ["input", "keyevent", intIn("keycode", input.keycode, 0, 400)];
    } else {
      throw new InputError("kind must be tap, swipe, text or keyevent");
    }
    await requireDevice(input.serial);
    await adb(["-s", input.serial, "shell", args.join(" ")]);
    return { serial: input.serial, kind: input.kind, sent: true };
  },

  async android_install(input) {
    const name = String(input.apk ?? "");
    const apk = path.resolve(APK_DIR, name);
    if (!apk.startsWith(APK_DIR + path.sep) || !apk.endsWith(".apk")) {
      throw new InputError(`apk must be a .apk file inside ${APK_DIR}`);
    }
    if (!existsSync(apk)) throw new InputError(`no such apk: ${name}`);
    const realApk = await realpath(apk);
    const realDir = await realpath(APK_DIR);
    if (!realApk.startsWith(realDir + path.sep)) {
      throw new InputError("apk resolves outside the apks directory (symlink)");
    }
    await requireDevice(input.serial);
    const out = (await adb(["-s", input.serial, "install", "-r", apk], { timeoutMs: 300000 })).trim();
    if (!/^Success/m.test(out)) throw new Error(`install did not succeed: ${out.slice(0, 300)}`);
    return { serial: input.serial, apk: name, result: out };
  },

  async android_logcat(input) {
    await requireDevice(input.serial);
    const lines = input.lines === undefined ? 200 : intIn("lines", input.lines, 1, 2000);
    const out = await adb(["-s", input.serial, "shell", `logcat -d -t ${lines}`]);
    return { serial: input.serial, lines, text: out };
  },

  async android_preflight() {
    const bin = findAdb();
    let adbVersion = null;
    try { adbVersion = (await adb(["version"])).split("\n")[0].trim(); } catch (e) { adbVersion = null; }
    const sdk = SDK_ROOT;
    const emu = sdk && existsSync(path.join(sdk, "emulator", "emulator")) ? path.join(sdk, "emulator", "emulator") : null;
    const kvm = existsSync("/dev/kvm");
    let kvmUsable = false;
    try { const fs = await import("node:fs"); fs.accessSync("/dev/kvm", fs.constants.R_OK | fs.constants.W_OK); kvmUsable = true; } catch { /* no access */ }
    let devices = [];
    if (adbVersion) { try { devices = (await tools.android_devices()).devices; } catch { /* reported below */ } }
    return {
      adb: { path: bin, version: adbVersion, ok: Boolean(adbVersion) },
      sdk_root: sdk, emulator_binary: emu, avdmanager: sdk ? existsSync(path.join(sdk, "cmdline-tools", "latest", "bin", "avdmanager")) : false,
      kvm: { present: kvm, usable_by_this_user: kvmUsable, note: kvm && !kvmUsable ? "add the user to the 'kvm' group (needs sudo); without KVM an x86 emulator is unusably slow" : undefined },
      policy: { shell_locked: SHELL_ALLOW.length === 0, shell_allow: SHELL_ALLOW, emulator_allowed: ALLOW_EMULATOR, token_required: Boolean(TOKEN_SECRET) },
      devices,
      ready_for_device: Boolean(adbVersion),
      ready_for_emulator: Boolean(adbVersion && emu && kvmUsable),
      termux_path: "Real-terminal feel on a stock phone: install Termux, run `pkg install openssh && sshd` (port 8022), then android_forward {serial, local:8022, remote:8022} and ssh -p 8022 localhost. No root needed.",
    };
  },

  async android_avd_list() {
    const sdk = SDK_ROOT;
    const emu = sdk && path.join(sdk, "emulator", "emulator");
    if (!emu || !existsSync(emu)) throw new Error("Android emulator not installed (set ANDROID_HOME to an SDK with the emulator package)");
    const { stdout } = await execFileP(emu, ["-list-avds"], { timeout: 15000 });
    return { avds: stdout.split(/\r?\n/).map((l) => l.trim()).filter(Boolean) };
  },

  async android_emulator_start(input) {
    if (!ALLOW_EMULATOR) throw new InputError("emulator control is LOCKED: set OZONE_ANDROID_ALLOW_EMULATOR=1");
    const avd = String(input.avd ?? "");
    if (!/^[A-Za-z0-9._-]{1,64}$/.test(avd)) throw new InputError("avd name required (see android_avd_list)");
    const pre = await tools.android_preflight();
    if (!pre.emulator_binary) throw new Error("Android emulator not installed");
    if (!pre.kvm.usable_by_this_user && input.allow_software !== true) throw new InputError("no usable /dev/kvm — an x86 emulator would be unusably slow; pass allow_software:true to try anyway");
    const { spawn } = await import("node:child_process");
    const child = spawn(pre.emulator_binary, ["-avd", avd, "-no-window", "-no-audio", "-no-snapshot", "-no-boot-anim", "-gpu", "swiftshader_indirect"], { detached: true, stdio: "ignore", env: { ...process.env, ANDROID_SDK_ROOT: SDK_ROOT, ...(JAVA_HOME ? { JAVA_HOME } : {}) } });
    child.unref();
    return { avd, pid: child.pid, headless: true, next: "poll android_devices until state=device, then android_ui_snapshot" };
  },

  async android_emulator_stop(input) {
    if (!ALLOW_EMULATOR) throw new InputError("emulator control is LOCKED: set OZONE_ANDROID_ALLOW_EMULATOR=1");
    if (!/^emulator-\d{4,5}$/.test(String(input.serial ?? ""))) throw new InputError("serial must be an emulator-NNNN serial (never a physical device)");
    await adb(["-s", input.serial, "emu", "kill"]);
    return { serial: input.serial, stopped: true };
  },

  // PERCEIVE: uiautomator dump -> numbered elements (same loop as browser_snapshot).
  // Text comes from the app under inspection = untrusted data, never instructions.
  async android_ui_snapshot(input) {
    await requireDevice(input.serial);
    const xml = await adb(["-s", input.serial, "exec-out", "uiautomator", "dump", "/dev/tty"], { timeoutMs: 30000 });
    const body = xml.slice(xml.indexOf("<?xml"));
    const els = [];
    for (const m of body.matchAll(/<node\b([^>]*?)\/?>/g)) {
      const attr = (k) => (m[1].match(new RegExp(`\\b${k}="([^"]*)"`)) ?? [])[1] ?? "";
      const b = attr("bounds").match(/\[(\d+),(\d+)\]\[(\d+),(\d+)\]/);
      if (!b) continue;
      const interactive = attr("clickable") === "true" || attr("long-clickable") === "true" || attr("checkable") === "true" || attr("scrollable") === "true";
      const label = attr("text") || attr("content-desc");
      if (!interactive && !label) continue;
      const [x1, y1, x2, y2] = b.slice(1).map(Number);
      els.push({ index: els.length, label: label.slice(0, 80), id: attr("resource-id"), class: attr("class").split(".").pop(), clickable: attr("clickable") === "true", bounds: [x1, y1, x2, y2], center: [Math.round((x1 + x2) / 2), Math.round((y1 + y2) / 2)] });
      if (els.length >= 150) break;
    }
    return { serial: input.serial, untrusted_elements: els, count: els.length };
  },

  async android_ui_tap_element(input) {
    const snap = await tools.android_ui_snapshot({ serial: input.serial });
    const el = input.index !== undefined ? snap.untrusted_elements[intIn("index", input.index, 0, 1000)]
      : snap.untrusted_elements.find((e) => input.text && e.label.toLowerCase().includes(String(input.text).toLowerCase()));
    if (!el) throw new InputError("no matching element (take android_ui_snapshot, then use index or text)");
    await tools.android_input({ serial: input.serial, kind: "tap", x: el.center[0], y: el.center[1] });
    return { tapped: { index: el.index, label: el.label, center: el.center } };
  },

  // TERMINAL-SENSE: a session that remembers cwd, so the agent works like a shell.
  // Every exec goes through the same policy as android_shell.
  async android_term_open(input) {
    await requireDevice(input.serial);
    const id = "t" + Math.random().toString(36).slice(2, 9);
    if (TERMS.size >= 4) throw new InputError("max 4 terminal sessions — android_term_close one");
    TERMS.set(id, { serial: input.serial, cwd: "/sdcard", at: Date.now() });
    return { term: id, cwd: "/sdcard", note: "stateless on the device side; cwd is tracked here. Same allowlist as android_shell." };
  },

  async android_term_exec(input) {
    const t = TERMS.get(String(input.term ?? ""));
    if (!t) throw new InputError("unknown terminal session (android_term_open first)");
    if (input.confirm !== true) throw new InputError("pass confirm:true to run a device command");
    const command = String(input.command ?? "").trim();
    if (!command || command.length > 2000) throw new InputError("command must be 1-2000 characters");
    const first = command.split(/\s+/)[0];
    if (first === "cd") {
      if (!SHELL_ALLOW.length) throw new InputError("terminal is LOCKED: OZONE_ANDROID_SHELL_ALLOW is not configured");
      const target = command.split(/\s+/)[1] ?? "/sdcard";
      const next = target.startsWith("/") ? target : path.posix.normalize(path.posix.join(t.cwd, target));
      if (!/^[A-Za-z0-9._\/-]{1,200}$/.test(next)) throw new InputError("invalid path");
      const chk = await adb(["-s", t.serial, "shell", `[ -d '${next}' ] && echo yes || echo no`]);
      if (!chk.includes("yes")) throw new InputError(`no such directory: ${next}`);
      t.cwd = next;
      return { term: input.term, cwd: t.cwd, stdout: "", exit_code: 0 };
    }
    shellPolicy(command);
    const out = await adb(["-s", t.serial, "shell", `cd '${t.cwd}' && ${command}\n__rc=$?; echo __OZONE_EXIT:$__rc`], { timeoutMs: 60000 });
    const m = out.match(/__OZONE_EXIT:(\d+)\s*$/);
    return { term: input.term, cwd: t.cwd, stdout: m ? out.slice(0, m.index) : out, exit_code: m ? Number(m[1]) : null };
  },

  async android_term_close(input) { return { closed: TERMS.delete(String(input.term ?? "")) }; },

  // Lets a human-installed Termux sshd (or similar) be reached from the host. Only
  // loopback forwards, only to a small set of ports; never `adb reverse`.
  async android_forward(input) {
    await requireDevice(input.serial);
    const local = intIn("local", input.local, 1024, 65535);
    const remote = intIn("remote", input.remote, 1024, 65535);
    if (![8022, 8080, 8081, 5555, 9222].includes(remote)) throw new InputError("remote port must be one of 8022 (Termux sshd), 8080, 8081, 5555, 9222");
    await adb(["-s", input.serial, "forward", `tcp:${local}`, `tcp:${remote}`]);
    return { serial: input.serial, forwarded: `127.0.0.1:${local} -> device:${remote}` };
  },

  async android_termux_status(input) {
    await requireDevice(input.serial);
    const out = await adb(["-s", input.serial, "shell", "pm path com.termux || true"]);
    return { serial: input.serial, installed: out.includes("package:"), how: "Install Termux from F-Droid, then `pkg install openssh; sshd`; use android_forward remote=8022." };
  },
};

function reply(res, code, body) {
  res.writeHead(code, { "Content-Type": "application/json" });
  res.end(JSON.stringify(body));
}

const server = createServer((req, res) => {
  if (req.method !== "POST" || !(req.url ?? "").startsWith("/call")) {
    reply(res, 404, { error: "POST /call only" });
    return;
  }
  if (TOKEN_SECRET && (req.headers["x-ozone-android-token"] ?? "") !== TOKEN_SECRET) {
    reply(res, 401, { success: false, error: "invalid or missing x-ozone-android-token" });
    return;
  }
  const chunks = [];
  req.on("data", (c) => chunks.push(c));
  req.on("end", async () => {
    let tool = "";
    let input = {};
    try {
      const body = JSON.parse(Buffer.concat(chunks).toString("utf8") || "{}");
      tool = String(body?.tool ?? "");
      input = body?.input ?? {};
    } catch {
      reply(res, 400, { success: false, error: "invalid JSON body" });
      return;
    }
    const fn = tools[tool];
    if (!fn) {
      reply(res, 400, { success: false, error: `unknown tool '${tool}'` });
      return;
    }
    try {
      const output = await fn(input);
      reply(res, 200, { success: true, output });
    } catch (err) {
      // 200 + success:false for every refusal/failure: the host drops the body of a non-2xx
      // answer ("tool returned HTTP 400"), which hid WHY a call was refused from the agent.
      reply(res, 200, { success: false, error: err.message });
    }
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") {
    console.error(`[android-mcp] :${PORT} in use — skipping`);
    return;
  }
  console.error(`[android-mcp] server error: ${err.message}`);
});

server.listen(PORT, "127.0.0.1", () => {
  console.error(`[android-mcp] /call on 127.0.0.1:${PORT} — android_preflight + devices/info/shell(locked)/screenshot/input/install/logcat + avd/emulator + ui_snapshot/tap_element + term_* + forward + termux_status (adb: ${findAdb()})`);
});
