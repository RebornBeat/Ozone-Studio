# browser-farm-mcp — Ozone as commander of Browser-Farm (Pattern B bridge)

Browser-Farm (`~/Projects/Browser-Farm`, FastAPI + Playwright + PostgreSQL + Xvfb) runs many isolated
browser profiles and has a singleton **COMMAND_CENTER** profile that coordinates the others through a
shared-state API. This MCP is the hook that lets Ozone-Studio sit above it. Doctrine, protocol and
assessment: `docs/BROWSER_AUTOMATION_GUIDE.md`.

Tools: `farm_status farm_profiles farm_profile farm_profile_start|pause|stop farm_navigate
farm_screenshot farm_metrics farm_state_get farm_state_set farm_command farm_result`.

**Deliberately not exposed:** script create/run (= arbitrary code on the farm), the account vault, proxy
registration/credentials, profile delete. Do those in the Browser-Farm client.

## Config (LOCKED until a farm + key are configured)
`OZONE_FARM_URL` + `OZONE_FARM_API_KEY_FILE` (preferred; the key never leaves this process) — or
`OZONE_FARM_SERVERS_FILE={"name":{"url":..,"api_key_file":..}}` for several farms (`server` in each call).
`OZONE_FARM_ALLOW_HOSTS` bounds `farm_navigate` (unset = refused). `OZONE_FARM_READONLY=1` disables every
mutating tool. Plain http is allowed only for loopback/private hosts unless `OZONE_FARM_ALLOW_INSECURE=1`.

## Command Center channel
`farm_command {target?, type, args}` writes `ozone~commands~<target>` into the farm's shared state;
`cc_script_ozone_bridge.py` (run it as a Script on the CC profile) consumes it and writes
`ozone~results~<target>`; `farm_result` reads it back. Handled types: `ping`, `echo`, `relay` (to a worker's
`farm~<id>~inbox`). Everything else is refused on the CC side too.

## Launch / tests
`node tools/browser-farm-mcp/server.mjs` (:3281) from the operator shell/supervisor, then
`node tools/register-browser-tools.mjs farm`.
`node tools/browser-farm-mcp/selftest.mjs` — 20 checks against a **mock** farm built from the real API
shapes. A live Browser-Farm run still needs PostgreSQL, Playwright, Xvfb on the farm machine.
