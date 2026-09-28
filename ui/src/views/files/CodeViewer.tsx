/**
 * F3 — Code viewer: read-only, syntax-aware source display for the file selected in the Files panel.
 *
 * Real sources rendered / integrated with (nothing here is sample or mock data):
 *  - Selected file:     ../../fileSelection `useSelectedFile()` (set by F1's browser / F2's reference viewer).
 *  - File content:      ../../data/fileContent `readFileContent(path)` (F0's guarded Electron IPC). Until that lands it
 *                       throws `FileContentUnavailable`; this view then says so plainly and shows NO source.
 *  - Graph gutter:      ./CodeNodeLinks `useCodeNodeLinks(projectId, path)` (F4) — real function/class graph nodes
 *                       covering line ranges; clicking a marker calls ../../navigation `navigateTo({kind:"graph-node"})`.
 *  - Deep links:        ../../navigation `takePendingNavigation("code-file")` / `onNavigate` (jump to `line`).
 *
 * Tokenizer: self-contained, single pass, per-line with carried state (block comments — nested for Rust —,
 * Python triple quotes, JS template literals, Rust raw strings / multi-line strings). No dependency added.
 * Keyword lists follow the Rust reference, ECMAScript reserved words, Python's `keyword.kwlist`; the "clike" spec is
 * a deliberate shared approximation for C/C++/Java/Go/C#/Kotlin/Swift/Scala/Dart. Known, accepted limits: JS regex
 * literals and `${}` interpolation inside template literals are not parsed (a template literal is one string token);
 * lines longer than MAX_LINE_CHARS are truncated for display (with an explicit note) and tokenised only up to there.
 *
 * Rendering is windowed (fixed line height, only visible lines + overscan are in the DOM), so large files
 * (upstream cap 2 MB) stay responsive.
 */
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useSelectedFile, SelectedFile } from "../../fileSelection";
import { FileContentResult, FileContentUnavailable, readFileContent } from "../../data/fileContent";
import { navigateTo, onNavigate, takePendingNavigation } from "../../navigation";
import { CodeNodeLink, useCodeNodeLinks } from "./CodeNodeLinks";

export type CodeViewerProps = { projectId: number | null };

// ─────────────────────────────────────────────────────────────────────────
// Tokenizer (pure — exported so it can be tested against real source files)
// ─────────────────────────────────────────────────────────────────────────

export type TokenKind = "kw" | "str" | "com" | "num" | "type" | "macro" | "plain" | "note";
export interface Token {
  k: TokenKind;
  t: string;
}
export type Lang = "rust" | "js" | "python" | "clike" | "plain";

interface LangSpec {
  lineComment: string[];
  block?: { open: string; close: string; nested: boolean };
  /** Single-line string delimiters. */
  quotes: string[];
  /** Strings that may span lines (python triple quotes, JS template literals, ...). */
  multiQuote: string[];
  /** Plain "..." strings may continue onto the next line (Rust). */
  multilineStrings: boolean;
  keywords: Set<string>;
  rust?: boolean;
  python?: boolean;
}

const words = (s: string): Set<string> => new Set(s.split(/\s+/).filter(Boolean));

const SPECS: Record<Exclude<Lang, "plain">, LangSpec> = {
  rust: {
    lineComment: ["//"],
    block: { open: "/*", close: "*/", nested: true },
    quotes: ['"'],
    multiQuote: [],
    multilineStrings: true,
    rust: true,
    keywords: words(
      "as async await break const continue crate dyn else enum extern false fn for if impl in let loop match mod move mut pub ref return self Self static struct super trait true type union unsafe use where while",
    ),
  },
  js: {
    lineComment: ["//"],
    block: { open: "/*", close: "*/", nested: false },
    quotes: ['"', "'"],
    multiQuote: ["`"],
    multilineStrings: false,
    keywords: words(
      "abstract as async await break case catch class const continue debugger declare default delete do else enum export extends false finally for from function get if implements import in instanceof interface keyof let namespace new null of package private protected public readonly return set static super switch this throw true try type typeof undefined var void while with yield",
    ),
  },
  python: {
    lineComment: ["#"],
    quotes: ['"', "'"],
    multiQuote: ['"""', "'''"],
    multilineStrings: false,
    python: true,
    keywords: words(
      "False None True and as assert async await break case class continue def del elif else except finally for from global if import in is lambda match nonlocal not or pass raise return self try while with yield",
    ),
  },
  clike: {
    lineComment: ["//"],
    block: { open: "/*", close: "*/", nested: false },
    quotes: ['"', "'"],
    multiQuote: ['"""', "`"],
    multilineStrings: false,
    keywords: words(
      "abstract auto bool break byte case catch chan char class const continue default defer do double else enum extends extern false fallthrough final finally float for fun func go goto if implements import instanceof int interface long map namespace native new nil null object override package private protected public range return select short signed sizeof static string struct switch synchronized this throw throws transient true try type typedef union unsigned using val var virtual void volatile when while",
    ),
  },
};

const EXT_LANG: Record<string, Lang> = {
  rs: "rust",
  ts: "js", tsx: "js", js: "js", jsx: "js", mjs: "js", cjs: "js", mts: "js", cts: "js",
  py: "python", pyw: "python",
  c: "clike", h: "clike", cc: "clike", cpp: "clike", cxx: "clike", hpp: "clike", hh: "clike",
  java: "clike", go: "clike", cs: "clike", kt: "clike", swift: "clike", scala: "clike", dart: "clike",
};

/** Extensions viewed as plain (unhighlighted) text but still treated as code-like. */
const PLAIN_CODELIKE = new Set(["json", "toml", "yaml", "yml", "sh", "bash", "zsh", "css", "scss", "html", "xml", "sql", "lock", "cfg", "ini", "env", "gitignore", "dockerfile", "makefile"]);

export function extOf(path: string | undefined): string {
  if (!path) return "";
  const base = path.split(/[\\/]/).pop() ?? "";
  const dot = base.lastIndexOf(".");
  return dot >= 0 ? base.slice(dot + 1).toLowerCase() : base.toLowerCase();
}
export function langForPath(path: string | undefined): Lang {
  return EXT_LANG[extOf(path)] ?? "plain";
}

const MAX_LINE_CHARS = 10000;

const IDENT = /[A-Za-z_$][A-Za-z0-9_$]*/y;
const NUM =
  /(?:0[xX][0-9a-fA-F_]+|0[bB][01_]+|0[oO][0-7_]+|\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+-]?\d+)?)(?:[a-zA-Z_][a-zA-Z0-9_]*)?/y;
const RUST_RAW = /b?r(#*)"/y;
const RUST_CHAR = /'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^\\'])'/y;
const RUST_LIFETIME = /'[A-Za-z_][A-Za-z0-9_]*/y;
const PY_PREFIX = /^(?:[rRbBfFuU]|[rR][bBfF]|[bBfF][rR])$/;

interface ScanState {
  mode: "code" | "block" | "multi";
  depth: number;
  close: string;
  escapes: boolean;
}

function tokenizeLine(full: string, spec: LangSpec, st: ScanState): Token[] {
  const truncated = full.length > MAX_LINE_CHARS;
  const line = truncated ? full.slice(0, MAX_LINE_CHARS) : full;
  const n = line.length;
  const out: Token[] = [];
  const push = (k: TokenKind, t: string) => {
    if (!t) return;
    const last = out[out.length - 1];
    if (last && last.k === k) last.t += t;
    else out.push({ k, t });
  };

  let i = 0;
  while (i < n) {
    if (st.mode === "block") {
      const blk = spec.block!;
      let j = i;
      while (j < n) {
        if (blk.nested && line.startsWith(blk.open, j)) {
          st.depth++;
          j += blk.open.length;
          continue;
        }
        if (line.startsWith(blk.close, j)) {
          j += blk.close.length;
          st.depth--;
          if (st.depth <= 0) {
            st.mode = "code";
            break;
          }
          continue;
        }
        j++;
      }
      push("com", line.slice(i, j));
      i = j;
      continue;
    }

    if (st.mode === "multi") {
      let j = i;
      let found = false;
      while (j < n) {
        if (st.escapes && line[j] === "\\") {
          j += 2;
          continue;
        }
        if (line.startsWith(st.close, j)) {
          j += st.close.length;
          found = true;
          break;
        }
        j++;
      }
      if (j > n) j = n;
      push("str", line.slice(i, j));
      i = j;
      if (found) st.mode = "code";
      continue;
    }

    const c = line[i];

    if (spec.block && line.startsWith(spec.block.open, i)) {
      st.mode = "block";
      st.depth = 1;
      push("com", spec.block.open);
      i += spec.block.open.length;
      continue;
    }
    let isLineComment = false;
    for (const lc of spec.lineComment) {
      if (line.startsWith(lc, i)) {
        isLineComment = true;
        break;
      }
    }
    if (isLineComment) {
      push("com", line.slice(i));
      i = n;
      break;
    }

    if (spec.rust && (c === "r" || c === "b")) {
      RUST_RAW.lastIndex = i;
      const m = RUST_RAW.exec(line);
      if (m) {
        st.mode = "multi";
        st.close = '"' + m[1];
        st.escapes = false;
        push("str", m[0]);
        i += m[0].length;
        continue;
      }
    }

    let matchedMulti = false;
    for (const mq of spec.multiQuote) {
      if (line.startsWith(mq, i)) {
        st.mode = "multi";
        st.close = mq;
        st.escapes = true;
        push("str", mq);
        i += mq.length;
        matchedMulti = true;
        break;
      }
    }
    if (matchedMulti) continue;

    if (spec.rust && c === "'") {
      RUST_CHAR.lastIndex = i;
      const cm = RUST_CHAR.exec(line);
      if (cm) {
        push("str", cm[0]);
        i += cm[0].length;
        continue;
      }
      RUST_LIFETIME.lastIndex = i;
      const lm = RUST_LIFETIME.exec(line);
      if (lm) {
        push("type", lm[0]);
        i += lm[0].length;
        continue;
      }
      push("plain", c);
      i++;
      continue;
    }

    if (spec.quotes.includes(c)) {
      let j = i + 1;
      let closed = false;
      while (j < n) {
        if (line[j] === "\\") {
          j += 2;
          continue;
        }
        if (line[j] === c) {
          j++;
          closed = true;
          break;
        }
        j++;
      }
      if (j > n) j = n;
      push("str", line.slice(i, j));
      i = j;
      if (!closed && spec.multilineStrings) {
        st.mode = "multi";
        st.close = c;
        st.escapes = true;
      }
      continue;
    }

    if (c >= "0" && c <= "9") {
      NUM.lastIndex = i;
      const m = NUM.exec(line);
      if (m) {
        push("num", m[0]);
        i += m[0].length;
        continue;
      }
    }

    IDENT.lastIndex = i;
    const im = IDENT.exec(line);
    if (im) {
      const id = im[0];
      const after = line[i + id.length];
      if (spec.python && PY_PREFIX.test(id) && (after === '"' || after === "'")) {
        push("str", id);
        i += id.length;
        continue;
      }
      if (spec.keywords.has(id)) push("kw", id);
      else if (spec.rust && after === "!" && line[i + id.length + 1] !== "=") {
        push("macro", id + "!");
        i += id.length + 1;
        continue;
      } else if (id.length > 1 && id[0] >= "A" && id[0] <= "Z") push("type", id);
      else push("plain", id);
      i += id.length;
      continue;
    }

    push("plain", c);
    i++;
  }

  if (truncated) out.push({ k: "note", t: ` … ${full.length - MAX_LINE_CHARS} more characters not shown` });
  return out;
}

export function splitLines(content: string): string[] {
  const lines = content.split(/\r?\n/);
  if (lines.length > 1 && lines[lines.length - 1] === "") lines.pop();
  return lines;
}

/** One token array per line (lossless: the tokens' text concatenates back to the line, up to MAX_LINE_CHARS). */
export function tokenizeLines(lines: string[], lang: Lang): Token[][] {
  if (lang === "plain") {
    return lines.map((l) =>
      l.length > MAX_LINE_CHARS
        ? [
            { k: "plain" as TokenKind, t: l.slice(0, MAX_LINE_CHARS) },
            { k: "note" as TokenKind, t: ` … ${l.length - MAX_LINE_CHARS} more characters not shown` },
          ]
        : [{ k: "plain" as TokenKind, t: l }],
    );
  }
  const spec = SPECS[lang];
  const st: ScanState = { mode: "code", depth: 0, close: "", escapes: true };
  return lines.map((l) => tokenizeLine(l, spec, st));
}

export function tokenizeSource(content: string, lang: Lang): Token[][] {
  return tokenizeLines(splitLines(content), lang);
}

// ─────────────────────────────────────────────────────────────────────────
// Presentation
// ─────────────────────────────────────────────────────────────────────────

const C_TEXT = "#dfe7f2";
const C_BODY = "#c7d0dc";
const C_MUTED = "#8b98ab";
const C_BORDER = "#1e2836";
const C_PANEL = "#0a0f1a";
const C_WARN = "#e8c14f";

const KIND_COLOR: Record<TokenKind, string> = {
  kw: "#c792ea",
  str: "#8fe38f",
  com: "#5c6b7f",
  num: "#ffb95f",
  type: "#5fb3ff",
  macro: "#e8c14f",
  plain: C_BODY,
  note: C_MUTED,
};
const NODE_COLOR: Record<string, string> = {
  Function: "#5fb3ff",
  Class: "#ffb95f",
  File: "#8fe38f",
  Import: "#9aa5b5",
};

const LH = 18;
const OVERSCAN = 25;
const MAX_MATCHES = 20000;
const MONO = "ui-monospace, SFMono-Regular, Menlo, Consolas, 'Liberation Mono', monospace";

interface Range {
  s: number;
  e: number;
  cur: boolean;
}

function renderTokens(tokens: Token[], ranges: Range[]): React.ReactNode[] {
  const styleOf = (k: TokenKind): React.CSSProperties => ({
    color: KIND_COLOR[k],
    fontStyle: k === "com" || k === "note" ? "italic" : undefined,
  });
  if (ranges.length === 0) {
    return tokens.map((tk, i) => (
      <span key={i} style={styleOf(tk.k)}>
        {tk.t}
      </span>
    ));
  }
  const out: React.ReactNode[] = [];
  let off = 0;
  let key = 0;
  for (const tk of tokens) {
    const tEnd = off + tk.t.length;
    let cursor = off;
    for (const r of ranges) {
      if (r.e <= cursor || r.s >= tEnd) continue;
      const s = Math.max(r.s, cursor);
      const e = Math.min(r.e, tEnd);
      if (s > cursor)
        out.push(
          <span key={key++} style={styleOf(tk.k)}>
            {tk.t.slice(cursor - off, s - off)}
          </span>,
        );
      out.push(
        <mark
          key={key++}
          style={{
            ...styleOf(tk.k),
            background: r.cur ? "rgba(232,193,79,0.65)" : "rgba(232,193,79,0.28)",
            borderRadius: 2,
          }}
        >
          {tk.t.slice(s - off, e - off)}
        </mark>,
      );
      cursor = e;
    }
    if (cursor < tEnd)
      out.push(
        <span key={key++} style={styleOf(tk.k)}>
          {tk.t.slice(cursor - off)}
        </span>,
      );
    off = tEnd;
  }
  return out;
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(2)} MB`;
}

type Load =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "error"; message: string; unavailable: boolean }
  | { kind: "ready"; result: FileContentResult };

interface Classification {
  isCode: boolean;
  message?: string;
  redirect?: string;
}

function classify(file: SelectedFile | null, path: string | undefined, lang: Lang): Classification {
  const modality = (file?.modality ?? "").toLowerCase();
  const ext = extOf(path ?? file?.name);
  if (modality === "math" || ext === "tex" || ext === "mathml")
    return { isCode: false, message: "This looks like a math file, not source code.", redirect: "Math" };
  if (modality === "text" || ["md", "markdown", "txt", "rst"].includes(ext))
    return { isCode: false, message: "This looks like a text document, not source code.", redirect: "Editor" };
  if (modality === "code" || lang !== "plain" || PLAIN_CODELIKE.has(ext)) return { isCode: true };
  return {
    isCode: false,
    message: ext ? `".${ext}" is not a recognised source-code type.` : "This file has no recognised source-code extension.",
    redirect: "Editor",
  };
}

const btn: React.CSSProperties = {
  background: "#101724",
  color: C_TEXT,
  border: `1px solid ${C_BORDER}`,
  borderRadius: 6,
  padding: "3px 9px",
  fontSize: 12,
  cursor: "pointer",
};
const input: React.CSSProperties = {
  background: "#101724",
  color: C_TEXT,
  border: `1px solid ${C_BORDER}`,
  borderRadius: 6,
  padding: "3px 8px",
  fontSize: 12.5,
};
const wrap: React.CSSProperties = { overflowWrap: "anywhere", wordBreak: "break-word", minWidth: 0 };

const Notice: React.FC<{ title: string; children?: React.ReactNode; tone?: "warn" | "muted" }> = ({ title, children, tone = "muted" }) => (
  <div
    style={{
      ...wrap,
      border: `1px solid ${tone === "warn" ? C_WARN : C_BORDER}`,
      borderRadius: 8,
      padding: "10px 12px",
      fontSize: 12.5,
      color: tone === "warn" ? C_WARN : C_MUTED,
      lineHeight: 1.6,
    }}
  >
    <div style={{ fontWeight: 700, color: tone === "warn" ? C_WARN : C_TEXT, marginBottom: 4 }}>{title}</div>
    {children}
  </div>
);

export const CodeViewer: React.FC<CodeViewerProps> = ({ projectId }) => {
  const [selectedFile] = useSelectedFile();

  // Deep-link target (navigation.ts "code-file"): may name a path other than the selected file.
  const [navTarget, setNavTarget] = useState<{ path: string; line?: number; projectId: number } | null>(null);
  useEffect(() => {
    const t = takePendingNavigation("code-file");
    if (t && t.kind === "code-file") setNavTarget({ path: t.path, line: t.line, projectId: t.projectId });
    return onNavigate((n) => {
      if (n.kind === "code-file") {
        takePendingNavigation("code-file");
        setNavTarget({ path: n.path, line: n.line, projectId: n.projectId });
      }
    });
  }, []);
  // Selecting a different file after arrival supersedes the deep link.
  const lastSelPath = useRef<string | undefined>(selectedFile?.path);
  useEffect(() => {
    if (selectedFile?.path !== lastSelPath.current) {
      lastSelPath.current = selectedFile?.path;
      setNavTarget(null);
    }
  }, [selectedFile?.path]);

  const path = navTarget?.path ?? selectedFile?.path;
  const navProjectId = projectId ?? selectedFile?.projectId ?? navTarget?.projectId ?? null;
  const lang = langForPath(path);
  const cls = useMemo(() => classify(navTarget ? null : selectedFile, path, lang), [selectedFile, navTarget, path, lang]);
  const [forceView, setForceView] = useState(false);
  useEffect(() => setForceView(false), [path]);

  const [load, setLoad] = useState<Load>({ kind: "idle" });
  useEffect(() => {
    if (!path) {
      setLoad({ kind: "idle" });
      return;
    }
    let cancelled = false;
    setLoad({ kind: "loading" });
    readFileContent(path)
      .then((result) => !cancelled && setLoad({ kind: "ready", result }))
      .catch((e) => {
        if (cancelled) return;
        setLoad({
          kind: "error",
          message: e instanceof Error ? e.message : String(e),
          unavailable: e instanceof FileContentUnavailable,
        });
      });
    return () => {
      cancelled = true;
    };
  }, [path]);

  const content = load.kind === "ready" ? load.result.content : "";
  const lines = useMemo(() => splitLines(content), [content]);
  const tokens = useMemo(() => tokenizeLines(lines, lang), [lines, lang]);
  const lineCount = lines.length;
  const digits = String(Math.max(lineCount, 1)).length;

  // Graph gutter markers (F4). Innermost (smallest range) link first per line.
  const links = useCodeNodeLinks(navProjectId, path);
  const coverage = useMemo(() => {
    const cov: (CodeNodeLink[] | undefined)[] = new Array(lineCount + 2);
    let budget = 400000;
    for (const l of links) {
      const end = Math.min(l.endLine, lineCount);
      for (let n = Math.max(1, l.startLine); n <= end && budget > 0; n++, budget--) (cov[n] ??= []).push(l);
    }
    for (const arr of cov) arr?.sort((a, b) => a.endLine - a.startLine - (b.endLine - b.startLine));
    return cov;
  }, [links, lineCount]);

  // Windowing
  const scrollerRef = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewH, setViewH] = useState(600);
  const isReady = load.kind === "ready";
  useEffect(() => {
    const el = scrollerRef.current;
    if (!el) return;
    setViewH(el.clientHeight || 600);
    if (typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver(() => setViewH(el.clientHeight || 600));
    ro.observe(el);
    return () => ro.disconnect();
  }, [isReady, cls.isCode, forceView]);
  useEffect(() => {
    setScrollTop(0);
    setHighlightLine(null);
    setQuery("");
  }, [path]);

  const [highlightLine, setHighlightLine] = useState<number | null>(null);
  const scrollToLine = useCallback((n: number) => {
    setHighlightLine(n);
    const el = scrollerRef.current;
    if (!el) return;
    const top = Math.max(0, (n - 1) * LH - el.clientHeight / 3);
    el.scrollTop = top;
    setScrollTop(top);
  }, []);

  // Deep-link line jump once content is on screen.
  useEffect(() => {
    if (isReady && navTarget?.line && lineCount > 0) scrollToLine(Math.min(Math.max(1, navTarget.line), lineCount));
  }, [isReady, navTarget, lineCount, scrollToLine]);

  // Search
  const [query, setQuery] = useState("");
  const [matchIdx, setMatchIdx] = useState(0);
  const ql = query.toLowerCase();
  const { matches, capped } = useMemo(() => {
    const m: { line: number; col: number }[] = [];
    if (!ql) return { matches: m, capped: false };
    for (let i = 0; i < lines.length; i++) {
      const lower = lines[i].toLowerCase();
      let from = 0;
      let at = lower.indexOf(ql, from);
      while (at >= 0) {
        m.push({ line: i + 1, col: at });
        if (m.length >= MAX_MATCHES) return { matches: m, capped: true };
        from = at + ql.length;
        at = lower.indexOf(ql, from);
      }
    }
    return { matches: m, capped: false };
  }, [lines, ql]);
  // A new query lands on its first match immediately.
  useEffect(() => {
    setMatchIdx(0);
    if (matches.length > 0) scrollToLine(matches[0].line);
  }, [matches, scrollToLine]);
  const gotoMatch = (idx: number) => {
    if (matches.length === 0) return;
    const k = ((idx % matches.length) + matches.length) % matches.length;
    setMatchIdx(k);
    scrollToLine(matches[k].line);
  };
  const currentMatch = matches.length > 0 ? matches[Math.min(matchIdx, matches.length - 1)] : null;

  // Jump to line
  const [jumpText, setJumpText] = useState("");
  const [jumpError, setJumpError] = useState<string | null>(null);
  const doJump = () => {
    const n = parseInt(jumpText, 10);
    if (!Number.isFinite(n) || n < 1 || n > lineCount) {
      setJumpError(`Enter a line from 1 to ${lineCount}`);
      return;
    }
    setJumpError(null);
    scrollToLine(n);
  };

  const fileName = selectedFile && !navTarget ? selectedFile.name : (path ?? "").split(/[\\/]/).pop() ?? "";

  // ── empty / gated states ─────────────────────────────────────────────
  if (!path) {
    return (
      <Notice title="No file selected">
        Pick a file in the <b>Browser</b> or <b>References</b> tab and it opens here. Only files linked into a project
        (graph-native file references) can be opened.
      </Notice>
    );
  }
  if (!cls.isCode && !forceView) {
    return (
      <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
        <div style={{ ...wrap, fontSize: 12.5, color: C_TEXT }}>
          <b>{fileName}</b> <span style={{ color: C_MUTED }}>{path}</span>
        </div>
        <Notice title="Not a code file">
          {cls.message} Open it in the <b>{cls.redirect}</b> sub-tab instead.
          <div style={{ marginTop: 8 }}>
            <button style={btn} onClick={() => setForceView(true)}>
              View as plain text anyway
            </button>
          </div>
        </Notice>
      </div>
    );
  }

  const first = Math.max(0, Math.floor(scrollTop / LH) - OVERSCAN);
  const lastLine = Math.min(lineCount, Math.ceil((scrollTop + Math.min(viewH, 2400)) / LH) + OVERSCAN);
  const rows: React.ReactNode[] = [];
  if (isReady) {
    for (let idx = first; idx < lastLine; idx++) {
      const n = idx + 1;
      const covers = coverage[n];
      const inner = covers?.[0];
      const color = inner ? NODE_COLOR[inner.nodeType] ?? "#c792ea" : undefined;
      const isHi = highlightLine === n;
      const ranges: Range[] = [];
      if (ql) {
        const lower = lines[idx].toLowerCase();
        let at = lower.indexOf(ql);
        while (at >= 0 && ranges.length < 200) {
          ranges.push({ s: at, e: at + ql.length, cur: !!currentMatch && currentMatch.line === n && currentMatch.col === at });
          at = lower.indexOf(ql, at + ql.length);
        }
      }
      rows.push(
        <div key={n} style={{ display: "flex", height: LH, lineHeight: `${LH}px`, background: isHi ? "rgba(232,193,79,0.14)" : undefined }}>
          <div
            style={{
              position: "sticky",
              left: 0,
              zIndex: 1,
              flex: "none",
              display: "flex",
              alignItems: "center",
              background: isHi ? "#1a1d18" : C_PANEL,
              borderRight: `1px solid ${C_BORDER}`,
              paddingRight: 8,
              marginRight: 10,
              color: isHi ? C_WARN : "#5c6b7f",
              userSelect: "none",
            }}
          >
            <button
              disabled={!inner || navProjectId === null}
              onClick={() => inner && navProjectId !== null && navigateTo({ kind: "graph-node", projectId: navProjectId, nodeId: inner.nodeId })}
              title={covers ? covers.map((l) => `${l.nodeType}: ${l.label} (lines ${l.startLine}–${l.endLine})`).join("\n") + "\nClick: open in Graph View" : undefined}
              style={{
                width: 14,
                height: LH,
                padding: 0,
                border: "none",
                borderLeft: color ? `3px solid ${color}` : "3px solid transparent",
                background: "transparent",
                color: color ?? "transparent",
                fontSize: 9,
                cursor: inner ? "pointer" : "default",
              }}
            >
              {inner && inner.startLine === n ? "◆" : ""}
            </button>
            <span style={{ minWidth: `${digits}ch`, textAlign: "right" }}>{n}</span>
          </div>
          <div style={{ flex: "none", whiteSpace: "pre", tabSize: 4 }}>{renderTokens(tokens[idx] ?? [], ranges)}</div>
        </div>,
      );
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 320, gap: 8 }}>
      <div style={{ ...wrap, fontSize: 12.5, color: C_BODY }}>
        <b style={{ color: C_TEXT }}>{fileName}</b> <span style={{ color: C_MUTED }}>{path}</span>
        {isReady && (
          <div style={{ fontSize: 11.5, color: C_MUTED, marginTop: 2 }}>
            {lang === "plain" ? "plain text" : lang === "js" ? "TypeScript/JavaScript" : lang === "clike" ? "C-family (approximate)" : lang[0].toUpperCase() + lang.slice(1)}
            {" · "}
            {lineCount} line{lineCount === 1 ? "" : "s"} · {formatBytes(load.result.sizeBytes)}
            {links.length > 0 && ` · ${links.length} graph node${links.length === 1 ? "" : "s"} linked`}
          </div>
        )}
      </div>

      {load.kind === "loading" && <div style={{ color: C_MUTED, fontSize: 12.5 }}>Loading {fileName}…</div>}
      {load.kind === "error" && (
        <Notice title={load.unavailable ? "File content is not available yet" : "Could not read this file"} tone="warn">
          <div style={wrap}>{load.message}</div>
          <div style={{ marginTop: 6, color: C_MUTED }}>
            {load.unavailable
              ? "The viewer only shows real file content; nothing is displayed in its place."
              : "The file may have moved, been deleted, or be outside what the host allows to be read."}
          </div>
        </Notice>
      )}

      {isReady && (
        <>
          {load.result.truncated && (
            <Notice title="Showing truncated content" tone="warn">
              The file is larger than the read cap; only the beginning ({formatBytes(load.result.content.length)}) is shown.
            </Notice>
          )}
          <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") gotoMatch(matchIdx + (e.shiftKey ? -1 : 1));
              }}
              placeholder="Search in file"
              style={{ ...input, width: 180 }}
              aria-label="Search in file"
            />
            <button style={btn} onClick={() => gotoMatch(matchIdx - 1)} disabled={matches.length === 0} title="Previous match (Shift+Enter)">
              ↑
            </button>
            <button style={btn} onClick={() => gotoMatch(matchIdx + 1)} disabled={matches.length === 0} title="Next match (Enter)">
              ↓
            </button>
            <span style={{ fontSize: 12, color: C_MUTED }}>
              {ql ? (matches.length === 0 ? "no matches" : `${Math.min(matchIdx, matches.length - 1) + 1} / ${matches.length}${capped ? "+ (capped)" : ""}`) : ""}
            </span>
            <span style={{ flex: 1 }} />
            <input
              value={jumpText}
              onChange={(e) => setJumpText(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && doJump()}
              placeholder="Line #"
              inputMode="numeric"
              style={{ ...input, width: 70 }}
              aria-label="Jump to line"
            />
            <button style={btn} onClick={doJump}>
              Go
            </button>
            {jumpError && <span style={{ fontSize: 12, color: "#ff8a8a" }}>{jumpError}</span>}
          </div>

          {lineCount === 0 ? (
            <Notice title="Empty file">This file has no content (0 bytes).</Notice>
          ) : (
            // maxHeight guards the windowing: without a definite ancestor height this scroller would otherwise grow to
            // the full virtual height and defeat virtualisation.
            <div
              ref={scrollerRef}
              onScroll={(e) => setScrollTop((e.target as HTMLDivElement).scrollTop)}
              style={{
                flex: 1,
                minHeight: 200,
                maxHeight: "78vh",
                overflow: "auto",
                background: C_PANEL,
                border: `1px solid ${C_BORDER}`,
                borderRadius: 8,
                fontFamily: MONO,
                fontSize: 12.5,
              }}
            >
              <div style={{ position: "relative", height: lineCount * LH + 12, minWidth: "100%" }}>
                <div style={{ position: "absolute", top: first * LH, left: 0, minWidth: "100%", width: "max-content" }}>{rows}</div>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
};

export default CodeViewer;
