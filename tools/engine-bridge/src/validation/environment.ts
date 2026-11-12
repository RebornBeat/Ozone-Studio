import { existsSync, statSync } from "node:fs";
import { delimiter, join } from "node:path";

export function envFlag(name: string) {
  const value = process.env[name]?.trim();
  return Boolean(value);
}

export function commandExists(command: string) {
  const pathValue = process.env.PATH;
  if (!pathValue) {
    return false;
  }

  const names = process.platform === "win32"
    ? [command, `${command}.exe`, `${command}.cmd`, `${command}.bat`]
    : [command];

  for (const basePath of pathValue.split(delimiter)) {
    for (const name of names) {
      const fullPath = join(basePath, name);
      try {
        if (!existsSync(fullPath)) {
          continue;
        }
        if (process.platform !== "win32") {
          const stat = statSync(fullPath);
          const isExecutable = (stat.mode & 0o111) !== 0;
          if (!isExecutable) {
            continue;
          }
        }
        return true;
      } catch {
        continue;
      }
    }
  }

  return false;
}
