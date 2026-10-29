"""Command-Center-side consumer for Ozone-Studio's browser-farm-mcp.

Run this as a Script on the Browser-Farm COMMAND_CENTER profile (Script Library ->
new script; Requirements: none). It uses only the namespace Browser-Farm injects
(get_state / set_state). Ozone writes COMMAND records to the farm's shared state
(via `farm_command`); this script consumes them and writes RESULT records back
(read by `farm_result`).

Wire (shared-state keys; '/' in a logical key is stored as '~'):
  ozone~commands~command-center   <- Ozone writes {id,type,args,issued_by,issued_at}
  ozone~results~command-center    -> this script writes {id,type,ok,output|error,finished_at}
  farm~<profile_id>~inbox         -> `relay` forwards args to a worker profile's script,
                                     which reads it with get_state("farm~<id>~inbox")

Command types handled (everything else is refused, never executed):
  ping    -> {"pong": true}
  echo    -> returns args
  relay   -> args {"profile_id": "...", "message": {...}}: write the worker's inbox
There is deliberately NO "run arbitrary code" command: scenario logic lives in
scripts the operator wrote in the Browser-Farm client, not in model-authored text.
"""
import asyncio
import datetime
import re

CMD_KEY = "ozone~commands~command-center"
RES_KEY = "ozone~results~command-center"
ID_RE = re.compile(r"^[A-Za-z0-9._:-]{1,80}$")


def _now():
    return datetime.datetime.utcnow().isoformat() + "Z"


async def _handle(cmd):
    t = cmd.get("type")
    args = cmd.get("args") or {}
    if t == "ping":
        return {"pong": True}
    if t == "echo":
        return {"args": args}
    if t == "relay":
        pid = str(args.get("profile_id", ""))
        msg = args.get("message")
        if not ID_RE.match(pid) or not isinstance(msg, dict):
            raise ValueError("relay needs profile_id (safe id) and message (object)")
        await set_state(f"farm~{pid}~inbox", {"from": "ozone", "command_id": cmd.get("id"), "message": msg, "at": _now()})
        return {"relayed_to": pid}
    raise ValueError(f"unknown command type '{t}' (ping | echo | relay)")


async def main(context):
    last_id = None
    while True:
        cmd = await get_state(CMD_KEY)
        if isinstance(cmd, dict) and cmd.get("id") and cmd["id"] != last_id:
            last_id = cmd["id"]
            try:
                out = await _handle(cmd)
                await set_state(RES_KEY, {"id": cmd["id"], "type": cmd.get("type"), "ok": True, "output": out, "finished_at": _now()})
            except Exception as e:  # report, never crash the CC loop
                await set_state(RES_KEY, {"id": cmd["id"], "type": cmd.get("type"), "ok": False, "error": str(e)[:300], "finished_at": _now()})
        await asyncio.sleep(2)
