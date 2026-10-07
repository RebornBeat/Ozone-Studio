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

function findAdb() {
  if (process.env.OZONE_ADB) return process.env.OZONE_ADB;
  if (process.env.ANDROID_HOME) {
    const p = path.join(process.env.ANDROID_HOME, "platform-tools", "adb");
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
    await requireDevice(input.serial);
    const command = String(input.command ?? "");
    if (!command.trim() || command.length > 2000) throw new InputError("command must be 1-2000 characters");
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
      const code = err instanceof InputError ? 400 : 500;
      reply(res, code, { success: false, error: err.message });
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
  console.error(`[android-mcp] /call on 127.0.0.1:${PORT} — android_devices, android_device_info, android_shell, android_screenshot, android_input, android_install, android_logcat (adb: ${findAdb()})`);
});
