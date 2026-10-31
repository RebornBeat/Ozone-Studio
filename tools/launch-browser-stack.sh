#!/usr/bin/env bash
# Browser stack launcher (2026-10-29, zcode, operator greenlight).
# Starts browser-mcp + browser-farm-mcp DETACHED (setsid) so they survive
# agent sessions; logs live with the host's data dir. Idempotent: kills any
# previous instance first. Run from anywhere:
#   bash tools/launch-browser-stack.sh
# browser-mcp :3280 — allowlist localhost, eval ON (UI session injection),
#                     no-sandbox opt-in (AppArmor userns restriction on this box)
# browser-farm-mcp :3281 — LOCKED until a farm is configured (OZONE_FARM_URL +
#                     OZONE_FARM_API_KEY_FILE); on the EU VPS run it configured.
# After starting, register tools into the host registry (persisted):
#   node /home/rebornbeat/Projects/Ozone-Studio/tools/register-browser-tools.mjs all
set -u
R="$(cd "$(dirname "$0")/.." && pwd)"
L="$R/target/release/zsei_data/logs"
mkdir -p "$L"

pkill -f "tools/browser-mcp/server.mjs" 2>/dev/null
pkill -f "tools/browser-farm-mcp/server.mjs" 2>/dev/null
pkill -f "tools/domorpher-agent/server.mjs" 2>/dev/null
pkill -f "tools/agent-mcp/server.mjs" 2>/dev/null
sleep 1

cd "$R"
setsid nohup env OZONE_BROWSER_ALLOW_HOSTS=localhost \
  OZONE_BROWSER_ALLOW_GLOBALS=OZONE_HOST_URL \
  OZONE_BROWSER_ALLOW_EVAL=1 \
  OZONE_BROWSER_NO_SANDBOX=1 \
  OZONE_BROWSER_PORT=3280 \
  node "$R/tools/browser-mcp/server.mjs" >> "$L/browser-mcp.log" 2>&1 &

setsid nohup env OZONE_FARM_PORT=3281 \
  node "$R/tools/browser-farm-mcp/server.mjs" >> "$L/browser-farm-mcp.log" 2>&1 &

# domorpher-agent is UNUSED for now (operator 2026-10-31: the Browser-Farm is driven via the
# Command Center through the farm tools). Opt-in only: OZONE_START_DOMORPHER_AGENT=1.
if [ "${OZONE_START_DOMORPHER_AGENT:-0}" = "1" ]; then
  setsid nohup env OZONE_DOMORPHER_PORT=3283 \
    node "$R/tools/domorpher-agent/server.mjs" >> "$L/agent-mcp.log" 2>&1 &
fi

sleep 2
echo "browser-mcp:   $(curl -s -m3 localhost:3280/status | head -c 120)"
echo "farm-mcp:      $(curl -s -m3 localhost:3281/status | head -c 120)"
[ "${OZONE_START_DOMORPHER_AGENT:-0}" = "1" ] && echo "domorpher-agent:  $(curl -s -m3 localhost:3283/status | head -c 120)" || echo "domorpher-agent:  not started (unused; OZONE_START_DOMORPHER_AGENT=1 to opt in)"

# android-mcp (adb at ~/Android/platform-tools). Shell is LOCKED unless OZONE_ANDROID_SHELL_ALLOW is set;
# emulator control needs OZONE_ANDROID_ALLOW_EMULATOR=1. Read-only allowlist below = safe inspection verbs.
R=/home/rebornbeat/Projects/Ozone-Studio
setsid nohup env ANDROID_HOME= OZONE_ADB=$HOME/Android/platform-tools/adb OZONE_ANDROID_PORT=3277 \
  OZONE_ANDROID_SHELL_ALLOW=getprop,ls,id,df,uname,ps,logcat,dumpsys,wm,ip \
  node $R/tools/android-mcp/server.mjs >> $R/target/release/zsei_data/logs/android-mcp.log 2>&1 &
