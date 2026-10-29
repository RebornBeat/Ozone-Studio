// terminal-mcp selftest — real server, inline roles; positive path + every bypass class refused.
import { spawn } from "node:child_process";
const PORT = 3291;
const roles = { tester: ["ls", "node", "python3", "find", "git", "npm", "systemctl", "echo"], "*": ["ls"] };
const srv = spawn("node", [new URL("./server.mjs", import.meta.url).pathname], { env: { ...process.env, OZONE_TERMINAL_PORT: String(PORT), OZONE_TERMINAL_ROLES: JSON.stringify(roles) }, stdio: "ignore" });
await new Promise((r) => setTimeout(r, 800));
let pass = 0, fail = 0;
const call = async (agent, command) => (await fetch(`http://127.0.0.1:${PORT}/call`, { method: "POST", body: JSON.stringify({ tool: "terminal_exec", agent, input: { command, cwd: "/tmp" } }) })).json();
const ok = (n, c, x = "") => { c ? pass++ : fail++; console.log(`${c ? "PASS" : "FAIL"}  ${n}${x ? "  — " + x : ""}`); };
let r = await call("tester", "ls /tmp");
ok("allowlisted command runs", r.success === true);
r = await call("tester", "node --version");
ok("node --version (harmless arg) runs", r.success === true, r.output?.stdout?.trim());
for (const [label, cmd] of [
  ["node -e", "node -e 1"], ["node --eval", "node --eval=1"], ["node -p", "node -p 1"],
  ["python3 -c", "python3 -c 1"], ["python3 -Sc", "python3 -Sc 1"],
  ["find -exec", "find /tmp -maxdepth 0 -exec ls {} +"], ["find -delete", "find /tmp/zz -delete"],
  ["git -c", "git -c core.pager=x log"], ["git config", "git config --global x y"],
  ["npm exec", "npm exec foo"], ["systemctl restart", "systemctl restart ssh"],
  ["operator ;", "ls; id"], ["operator $()", "ls $(id)"], ["newline", "ls\nid"],
]) { r = await call("tester", cmd); ok(`refused: ${label}`, r.success === false && /refused|rejected|not in the allowlist/.test(r.error ?? ""), r.error?.slice(0, 70)); }
r = await call("stranger", "node --version");
ok("'*' tier only gets ls", r.success === false && /not in the allowlist/.test(r.error));
r = await call("tester", "nodemon x");
ok("exact first-token match (nodemon != node)", r.success === false);
srv.kill();
console.log(`\n${pass} passed, ${fail} failed`); process.exit(fail ? 1 : 0);
