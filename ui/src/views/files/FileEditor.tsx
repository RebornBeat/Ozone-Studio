/**
 * F5 — File editor (Files panel → "Editor" tab).
 *
 * Real read/write for the file currently selected in the Files panel
 * (`useSelectedFile()`, ../../fileSelection — set by F1's browser). All disk I/O goes
 * through `readFileContent` / `writeFileContent` (../../data/fileContent, fork F0's guarded
 * Electron IPC). Neither the preload nor the host had any file read/write before F0, so when
 * F0's bridge is absent (browser dev path, or F0 not landed) the editor is DISABLED with an
 * explanation — it never shows a fake or empty buffer.
 *
 * This component WRITES user files, so the safety behaviours are deliberate:
 *  - No autosave. Save happens only on the Save button or Ctrl/Cmd+S (both explicit).
 *  - Optimistic concurrency: Save passes the `mtimeMs` captured at the last read/save as
 *    `expectedMtimeMs`. If the file changed on disk the write is refused and the editor
 *    shows a conflict banner (Reload from disk / Keep my edits / Copy my edits). There is
 *    intentionally NO "overwrite anyway" button. After a conflict, Save stays disabled
 *    until the user reloads, so a stale buffer can never be written by accident.
 *  - If a read returned no mtime, external changes cannot be detected; Save is disabled until
 *    the user explicitly acknowledges that.
 *  - A `truncated:true` read is shown read-only and can never be saved (a partial buffer must
 *    not be written back). NUL bytes in the content mark the file non-text: read-only.
 *  - Only `kind:"file"` selections with a recorded path and a text-like modality are editable.
 *    Modality strings are the ones file_link's `detect_modality` assigns
 *    (assets/pipelines/general/file_link/main.rs:260): Code/Text/Data/Config/Web/Math/Unknown are
 *    editable; Document/Image/Audio/Video/Archive/Chemistry/DNA/EEG are refused.
 *  - Unsaved-changes guards: selecting a different file while dirty does NOT swap the buffer —
 *    a banner offers "Discard my edits and open X" / "Keep editing"; Reload from disk needs a
 *    second click when dirty; a `beforeunload` prompt fires while dirty.
 *
 * Editing decisions (documented per the directive): a plain <textarea> (no editor library is
 * installed and package.json is off-limits). Tab INSERTS two spaces by default (code editing);
 * the "Tab inserts spaces" checkbox turns that off so Tab moves focus again (keyboard
 * accessibility escape hatch). Insertion uses execCommand("insertText") so native undo keeps
 * working, with a setRange fallback. `takePendingNavigation("code-file")` is deliberately NOT
 * consumed here: the code viewer (F3) is the natural owner of that target and stealing it
 * would break jump-to-source.
 *
 * Re-analysis: after a save, graphs derived from this file are NOT re-analysed. There is no
 * route to offer: file_link analyses only when a file is linked (the `analyze` flag on `Link`),
 * `Refresh` updates only exists/size/modified, and linking an already-linked path is rejected
 * ("File already linked to this project"). The UI states this plainly instead of implying
 * the Context Viewer will update.
 */
import React, { useDeferredValue, useEffect, useMemo, useRef, useState } from "react";
import { SelectedFile, useSelectedFile } from "../../fileSelection";
import { FileContentUnavailable, readFileContent, writeFileContent } from "../../data/fileContent";

export type FileEditorProps = { projectId: number | null };

const C = {
  text: "var(--color-text)",
  body: "var(--color-text-secondary)",
  muted: "var(--color-text-muted)",
  border: "var(--color-border-faint)",
  panel: "var(--color-bg)",
  warn: "#e8c14f",
  err: "#ff8a8a",
  ok: "#8fe38f",
};

const TEXT_MODALITIES = new Set(["code", "text", "data", "config", "web", "math", "unknown"]);

function refusalFor(f: SelectedFile): string | null {
  if (f.kind !== "file") return `A ${f.kind} reference has no local file to edit.`;
  if (!f.path) return "This file reference has no recorded local path.";
  const m = (f.modality ?? "").toLowerCase();
  if (m && !TEXT_MODALITIES.has(m)) {
    return `Modality "${f.modality}" is not a text format — binary and structured files are read-only here.`;
  }
  return null;
}

const sameFile = (a: SelectedFile, b: SelectedFile) => a.containerId === b.containerId && a.path === b.path;

function isUnavailable(e: unknown): boolean {
  return e instanceof FileContentUnavailable || (e instanceof Error && e.name === "FileContentUnavailable");
}
const errMessage = (e: unknown) => (e instanceof Error ? e.message : String(e));
const CONFLICT_RE = /mtime|modified|changed (on|since)|conflict|stale|expected/i;

const fmtBytes = (n: number) => (n < 1024 ? `${n} B` : n < 1048576 ? `${(n / 1024).toFixed(1)} KB` : `${(n / 1048576).toFixed(2)} MB`);
const fmtTime = (ms: number | undefined) => (ms === undefined ? "unknown" : new Date(ms).toLocaleString());

type Load =
  | { kind: "none" }
  | { kind: "refused"; reason: string }
  | { kind: "loading" }
  | { kind: "unavailable"; message: string }
  | { kind: "error"; message: string }
  | { kind: "ready" };

type Save =
  | { kind: "idle" }
  | { kind: "saving" }
  | { kind: "saved"; mtimeMs: number }
  | { kind: "conflict"; message: string }
  | { kind: "error"; message: string };

interface Doc {
  path: string;
  original: string;
  mtimeMs: number | undefined;
  sizeBytes: number;
  truncated: boolean;
  binary: boolean;
}

const btn = (enabled: boolean, primary = false): React.CSSProperties => ({
  background: primary && enabled ? "#1f6feb" : "transparent",
  color: enabled ? (primary ? "#fff" : C.text) : C.muted,
  border: `1px solid ${primary && enabled ? "#1f6feb" : C.border}`,
  borderRadius: 6,
  padding: "4px 12px",
  fontSize: 12,
  cursor: enabled ? "pointer" : "not-allowed",
  opacity: enabled ? 1 : 0.6,
});

const Banner: React.FC<{ tone: "warn" | "err" | "ok" | "info"; children: React.ReactNode }> = ({ tone, children }) => {
  const color = tone === "warn" ? C.warn : tone === "err" ? C.err : tone === "ok" ? C.ok : C.muted;
  return (
    <div
      role={tone === "err" || tone === "warn" ? "alert" : "status"}
      style={{
        border: `1px solid ${color}`,
        color,
        borderRadius: 6,
        padding: "6px 10px",
        marginBottom: 8,
        fontSize: 12,
        lineHeight: 1.5,
        overflowWrap: "anywhere",
      }}
    >
      {children}
    </div>
  );
};

export const FileEditor: React.FC<FileEditorProps> = () => {
  const [selected] = useSelectedFile();

  const [openFile, setOpenFile] = useState<SelectedFile | null>(null);
  const [doc, setDoc] = useState<Doc | null>(null);
  const [buffer, setBuffer] = useState("");
  const [load, setLoad] = useState<Load>({ kind: "none" });
  const [save, setSave] = useState<Save>({ kind: "idle" });
  const [stale, setStale] = useState(false);
  const [pendingSwitch, setPendingSwitch] = useState<SelectedFile | null>(null);
  const [confirm, setConfirm] = useState<null | "reload" | "switch">(null);
  const [ackNoMtime, setAckNoMtime] = useState(false);
  const [wrap, setWrap] = useState(false);
  const [tabInserts, setTabInserts] = useState(true);
  const [cursor, setCursor] = useState({ line: 1, col: 1 });
  const [copied, setCopied] = useState(false);

  const taRef = useRef<HTMLTextAreaElement>(null);
  const loadToken = useRef(0);
  const dirty = doc !== null && buffer !== doc.original;
  const dirtyRef = useRef(false);
  dirtyRef.current = dirty;

  const deferredBuffer = useDeferredValue(buffer);
  const bufferBytes = useMemo(() => new TextEncoder().encode(deferredBuffer).length, [deferredBuffer]);
  const lineCount = useMemo(() => {
    let n = 1;
    for (let i = deferredBuffer.indexOf("\n"); i !== -1; i = deferredBuffer.indexOf("\n", i + 1)) n++;
    return n;
  }, [deferredBuffer]);

  async function openDocument(sel: SelectedFile) {
    const token = ++loadToken.current;
    setPendingSwitch(null);
    setConfirm(null);
    setOpenFile(sel);
    setSave({ kind: "idle" });
    setStale(false);
    setAckNoMtime(false);
    setCopied(false);
    const refusal = refusalFor(sel);
    if (refusal) {
      setDoc(null);
      setBuffer("");
      setLoad({ kind: "refused", reason: refusal });
      return;
    }
    setLoad({ kind: "loading" });
    try {
      const r = await readFileContent(sel.path as string);
      if (token !== loadToken.current) return;
      const binary = r.content.includes("\u0000");
      setDoc({
        path: r.path,
        original: r.content,
        mtimeMs: r.mtimeMs,
        sizeBytes: r.sizeBytes,
        truncated: r.truncated === true,
        binary,
      });
      setBuffer(r.content);
      setLoad({ kind: "ready" });
    } catch (e) {
      if (token !== loadToken.current) return;
      setDoc(null);
      setBuffer("");
      setLoad(isUnavailable(e) ? { kind: "unavailable", message: errMessage(e) } : { kind: "error", message: errMessage(e) });
    }
  }

  // Selection changed elsewhere (F1/F2): open it, unless that would discard unsaved edits.
  useEffect(() => {
    if (!selected) return;
    if (openFile && sameFile(openFile, selected)) return;
    if (dirtyRef.current) {
      setPendingSwitch(selected);
      return;
    }
    void openDocument(selected);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selected]);

  useEffect(
    () => () => {
      loadToken.current++;
    },
    [],
  );

  useEffect(() => {
    if (!dirty) return;
    const h = (e: BeforeUnloadEvent) => {
      e.preventDefault();
      e.returnValue = "";
    };
    window.addEventListener("beforeunload", h);
    return () => window.removeEventListener("beforeunload", h);
  }, [dirty]);

  const readOnly = !doc || doc.truncated || doc.binary;
  const mtimeKnown = doc?.mtimeMs !== undefined;
  const canSave =
    load.kind === "ready" &&
    doc !== null &&
    !readOnly &&
    dirty &&
    !stale &&
    save.kind !== "saving" &&
    (mtimeKnown || ackNoMtime);

  async function doSave() {
    if (!canSave || !doc) return;
    setSave({ kind: "saving" });
    try {
      const res = await writeFileContent(doc.path, buffer, doc.mtimeMs);
      setDoc({ ...doc, original: buffer, mtimeMs: res.mtimeMs, sizeBytes: new TextEncoder().encode(buffer).length });
      setSave({ kind: "saved", mtimeMs: res.mtimeMs });
    } catch (e) {
      const message = errMessage(e);
      if (isUnavailable(e)) {
        setSave({ kind: "error", message });
      } else if (CONFLICT_RE.test(message)) {
        setStale(true);
        setSave({ kind: "conflict", message });
      } else {
        setSave({ kind: "error", message });
      }
    }
  }

  function reload() {
    if (!openFile) return;
    if (dirty && confirm !== "reload") {
      setConfirm("reload");
      return;
    }
    void openDocument(openFile);
  }

  function updateCursor(el: HTMLTextAreaElement) {
    const pos = el.selectionStart;
    const before = el.value.slice(0, pos);
    const lastNl = before.lastIndexOf("\n");
    let line = 1;
    for (let i = before.indexOf("\n"); i !== -1; i = before.indexOf("\n", i + 1)) line++;
    setCursor({ line, col: pos - lastNl });
  }

  function onKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
      e.preventDefault();
      void doSave();
      return;
    }
    if (e.key === "Tab" && tabInserts && !readOnly && !e.shiftKey && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault();
      const el = e.currentTarget;
      if (!document.execCommand("insertText", false, "  ")) {
        const s = el.selectionStart;
        const en = el.selectionEnd;
        setBuffer(buffer.slice(0, s) + "  " + buffer.slice(en));
        requestAnimationFrame(() => {
          el.selectionStart = el.selectionEnd = s + 2;
        });
      }
    }
  }

  async function copyEdits() {
    try {
      await navigator.clipboard.writeText(buffer);
      setCopied(true);
    } catch {
      setCopied(false);
      taRef.current?.focus();
      taRef.current?.select();
    }
  }

  // ── render ────────────────────────────────────────────────────────────
  if (!selected && !openFile) {
    return (
      <div style={{ padding: 16, color: C.muted, fontSize: 12.5 }}>
        No file selected. Pick a file in the Files panel's Browser or References tab to edit it here.
      </div>
    );
  }

  const showEditor = load.kind === "ready" && doc !== null;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 0, color: C.body, fontSize: 12.5, minWidth: 0 }}>
      <div style={{ display: "flex", flexWrap: "wrap", gap: "4px 14px", alignItems: "baseline", marginBottom: 8 }}>
        <span style={{ color: C.text, fontWeight: 700, overflowWrap: "anywhere" }}>{openFile?.name ?? "—"}</span>
        {dirty && (
          <span style={{ color: C.warn, fontWeight: 700 }} title="Buffer differs from what is on disk">
            ● unsaved changes
          </span>
        )}
        {openFile?.path && <span style={{ color: C.muted, overflowWrap: "anywhere" }}>{openFile.path}</span>}
      </div>

      {pendingSwitch && (
        <Banner tone="warn">
          You selected <b>{pendingSwitch.name}</b> but this buffer has unsaved edits to <b>{openFile?.name}</b>.
          <div style={{ display: "flex", gap: 8, marginTop: 6, flexWrap: "wrap" }}>
            <button
              style={btn(true)}
              onClick={() => {
                if (confirm !== "switch") setConfirm("switch");
                else void openDocument(pendingSwitch);
              }}
            >
              {confirm === "switch" ? "Really discard my edits and open it" : `Discard my edits and open ${pendingSwitch.name}`}
            </button>
            <button
              style={btn(true)}
              onClick={() => {
                setPendingSwitch(null);
                setConfirm(null);
              }}
            >
              Keep editing {openFile?.name}
            </button>
          </div>
        </Banner>
      )}

      {load.kind === "loading" && <div style={{ color: C.muted }}>Reading file from disk…</div>}
      {load.kind === "refused" && <Banner tone="info">Not editable: {load.reason}</Banner>}
      {load.kind === "unavailable" && (
        <Banner tone="warn">
          File content access isn't available, so the editor is disabled rather than showing an empty buffer.
          <div style={{ marginTop: 4, color: C.muted }}>{load.message}</div>
          <div style={{ marginTop: 4, color: C.muted }}>
            Reading and writing local files requires the desktop app's guarded file bridge; the browser-only dev build
            has none.
          </div>
        </Banner>
      )}
      {load.kind === "error" && (
        <Banner tone="err">
          Could not read the file: {load.message}
          <div style={{ marginTop: 6 }}>
            <button style={btn(true)} onClick={() => openFile && void openDocument(openFile)}>
              Try again
            </button>
          </div>
        </Banner>
      )}

      {showEditor && doc && (
        <>
          {doc.truncated && (
            <Banner tone="warn">
              The file was larger than the read limit, so only part of it was loaded. It is shown read-only and cannot
              be saved — writing a truncated buffer back would destroy the rest of the file.
            </Banner>
          )}
          {doc.binary && (
            <Banner tone="warn">
              This file contains NUL bytes, so it is treated as binary and shown read-only.
            </Banner>
          )}
          {!mtimeKnown && !readOnly && (
            <Banner tone="warn">
              The read returned no modification time, so changes made to this file elsewhere cannot be detected. Saving
              would overwrite them.
              <label style={{ display: "flex", gap: 6, alignItems: "center", marginTop: 6, cursor: "pointer" }}>
                <input type="checkbox" checked={ackNoMtime} onChange={(e) => setAckNoMtime(e.target.checked)} style={{ margin: 0 }} />
                I understand — allow saving anyway
              </label>
            </Banner>
          )}
          {save.kind === "conflict" && (
            <Banner tone="err">
              Not saved — the file changed on disk since you opened it. {save.message}
              <div style={{ display: "flex", gap: 8, marginTop: 6, flexWrap: "wrap" }}>
                <button style={btn(true)} onClick={reload}>
                  {confirm === "reload" ? "Really discard my edits and reload" : "Reload from disk (discards my edits)"}
                </button>
                <button style={btn(true)} onClick={() => setSave({ kind: "idle" })}>
                  Keep my edits
                </button>
                <button style={btn(true)} onClick={() => void copyEdits()}>
                  {copied ? "Copied" : "Copy my edits"}
                </button>
              </div>
              <div style={{ marginTop: 6, color: C.muted }}>
                Saving stays disabled until you reload, so an out-of-date buffer can't overwrite the newer file.
              </div>
            </Banner>
          )}
          {save.kind === "error" && <Banner tone="err">Save failed: {save.message}</Banner>}
          {save.kind === "saved" && (
            <Banner tone="ok">
              Saved. New modification time: {fmtTime(save.mtimeMs)}.
              <div style={{ marginTop: 4, color: C.muted }}>
                Graphs derived from this file (Graph View, engines) are NOT re-analysed automatically — file_link only
                analyses when a file is linked, and there is no re-analyse route.
              </div>
            </Banner>
          )}
          {confirm === "reload" && save.kind !== "conflict" && (
            <Banner tone="warn">
              Reloading discards your unsaved edits. Click "Reload from disk" again to confirm.
            </Banner>
          )}

          <div style={{ display: "flex", flexWrap: "wrap", gap: 8, alignItems: "center", marginBottom: 8 }}>
            <button style={btn(canSave, true)} disabled={!canSave} onClick={() => void doSave()} title="Save (Ctrl/Cmd+S)">
              {save.kind === "saving" ? "Saving…" : "Save"}
            </button>
            <button style={btn(load.kind === "ready")} onClick={reload}>
              Reload from disk
            </button>
            <label style={{ display: "flex", gap: 5, alignItems: "center", color: C.muted, cursor: "pointer" }}>
              <input type="checkbox" checked={wrap} onChange={(e) => setWrap(e.target.checked)} style={{ margin: 0 }} />
              Wrap lines
            </label>
            <label
              style={{ display: "flex", gap: 5, alignItems: "center", color: C.muted, cursor: "pointer" }}
              title="When on, the Tab key inserts two spaces; turn off to let Tab move keyboard focus."
            >
              <input type="checkbox" checked={tabInserts} onChange={(e) => setTabInserts(e.target.checked)} style={{ margin: 0 }} />
              Tab inserts spaces
            </label>
            {readOnly && <span style={{ color: C.warn }}>read-only</span>}
          </div>

          <textarea
            ref={taRef}
            value={buffer}
            readOnly={readOnly}
            wrap={wrap ? "soft" : "off"}
            spellCheck={false}
            autoCapitalize="off"
            autoCorrect="off"
            rows={26}
            onChange={(e) => {
              setBuffer(e.target.value);
              if (save.kind === "saved" || save.kind === "error") setSave({ kind: "idle" });
              updateCursor(e.target);
            }}
            onKeyDown={onKeyDown}
            onKeyUp={(e) => updateCursor(e.currentTarget)}
            onClick={(e) => updateCursor(e.currentTarget)}
            onSelect={(e) => updateCursor(e.currentTarget)}
            aria-label={`Contents of ${openFile?.name ?? "file"}`}
            style={{
              width: "100%",
              boxSizing: "border-box",
              minHeight: 320,
              resize: "vertical",
              background: C.panel,
              color: C.text,
              border: `1px solid ${C.border}`,
              borderRadius: 8,
              padding: 10,
              fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
              fontSize: 12.5,
              lineHeight: 1.5,
              whiteSpace: wrap ? "pre-wrap" : "pre",
              overflowX: wrap ? "hidden" : "auto",
              tabSize: 2,
              opacity: readOnly ? 0.8 : 1,
            }}
          />

          <div style={{ display: "flex", flexWrap: "wrap", gap: "2px 16px", marginTop: 6, color: C.muted, fontSize: 11.5 }}>
            <span>
              Ln {cursor.line}, Col {cursor.col}
            </span>
            <span>{lineCount.toLocaleString()} lines</span>
            <span>{buffer.length.toLocaleString()} chars</span>
            <span>
              {dirty ? `${fmtBytes(bufferBytes)} in buffer · ${fmtBytes(doc.sizeBytes)} on disk` : fmtBytes(doc.sizeBytes)}
            </span>
            <span>modified on disk: {fmtTime(doc.mtimeMs)}</span>
            <span title="Absolute path as recorded by file_link">{doc.path}</span>
          </div>
        </>
      )}
    </div>
  );
};

export default FileEditor;
