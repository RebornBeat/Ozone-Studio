/**
 * F0 — real file read/write for files linked into a project, via guarded
 * Electron IPC (`ui/electron/main.js` + `preload.js`, both owned by this
 * fork alongside this file).
 *
 * Main.js refuses anything that isn't a REAL, currently-registered
 * `FileReference` container's path — it re-confirms registration against
 * the live host on every call (see main.js's `findRegisteredFileContainerId`
 * doc comment for exactly how, since `file_link`'s containers have no
 * dedicated by-path lookup). This module is a thin, honest wrapper: no
 * filesystem access exists at all outside Electron (a plain browser dev
 * session has no `window.ozone.files`), so every function throws
 * `FileContentUnavailable` in that case rather than pretending to work.
 */
export interface FileContentResult {
  path: string;
  content: string;
  sizeBytes: number;
  mtimeMs?: number;
  truncated?: boolean;
}
export class FileContentUnavailable extends Error {}

function filesBridge(): { read: (p: string) => Promise<FileContentResult>; write: (p: string, c: string, m?: number) => Promise<{ ok: true; mtimeMs: number }> } | null {
  const bridge = (window as any)?.ozone?.files;
  return bridge && typeof bridge.read === "function" && typeof bridge.write === "function" ? bridge : null;
}

export async function readFileContent(path: string): Promise<FileContentResult> {
  const bridge = filesBridge();
  if (!bridge) {
    throw new FileContentUnavailable(
      "File content access requires the desktop app (no window.ozone.files bridge in this session).",
    );
  }
  try {
    return await bridge.read(path);
  } catch (e) {
    throw new FileContentUnavailable(e instanceof Error ? e.message : String(e));
  }
}

/** `expectedMtimeMs` = optimistic-concurrency check: refuse to overwrite a file changed since it was read. */
export async function writeFileContent(
  path: string,
  content: string,
  expectedMtimeMs?: number,
): Promise<{ ok: true; mtimeMs: number }> {
  const bridge = filesBridge();
  if (!bridge) {
    throw new FileContentUnavailable(
      "File content access requires the desktop app (no window.ozone.files bridge in this session).",
    );
  }
  try {
    return await bridge.write(path, content, expectedMtimeMs);
  } catch (e) {
    throw new FileContentUnavailable(e instanceof Error ? e.message : String(e));
  }
}
