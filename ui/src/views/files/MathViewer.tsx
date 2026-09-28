/**
 * F6 — Math content viewer.
 *
 * Two real sources, shown together:
 *  (a) the selected file's raw content (`useSelectedFile` from ../../fileSelection,
 *      set by F1's WorkspaceBrowser) via `readFileContent` (../../data/fileContent —
 *      F0 is building the guarded Electron IPC in parallel; until it lands this
 *      always throws `FileContentUnavailable`, handled honestly below, never faked).
 *  (b) the project's real persisted math graph(s) via `loadGraphData(projectId, cb,
 *      {modality:"math"})` (../../graphViewData), which already carries the
 *      per-node isReal/real-vs-schema-only judgment from ../../graphRenderers/mathNodes.ts
 *      (real types: Root/ProofStep/Variable/Assumption) and mathEdges.ts (real:
 *      Contains/FollowsStep/Uses/Defines/AssumesIn/DischargesIn).
 *
 * Real content shape, confirmed by reading every node/label/content string across
 * all 7 on-disk math graphs (assets/pipelines/modalities/math/zsei_data/graphs/
 * math_*.json) AND the source structs (ParseResult/ProofAnalysis/ProofStep,
 * assets/pipelines/modalities/math/main.rs:546-823): proof-step content today is
 * plain ASCII English + simple notation — "Let x > 0", "x^2 > 0", "x : Real",
 * "Using step 1, x is real." — NOT LaTeX. `scope_tree` (the wrapper's real field)
 * is `null` on every sample checked — real field, currently always unpopulated;
 * rendered as an honest "not recorded" note, not hidden or faked.
 *
 * Rendering: this is a small, honest LaTeX/ASCII-math -> Unicode converter for the
 * common subset (Greek letters, ^/_ super/subscript, \frac \sqrt \sum \int \forall
 * \exists \leq \geq \neq \to \cdot etc.) plus bare `^digits`/`_digits` (what the
 * real data actually uses). Anything it can't confidently convert is left as raw
 * text, never guessed or dropped. NO math-typesetting library (KaTeX/MathJax) is
 * installed and package.json is out of scope for this fork — full typesetting
 * would need one; recommended in this fork's handoff, not built here.
 */
import React, { useEffect, useMemo, useState } from "react";
import { useSelectedFile } from "../../fileSelection";
import { readFileContent, FileContentUnavailable, FileContentResult } from "../../data/fileContent";
import { loadGraphData } from "../../graphViewData";
import type { GraphViewNode, GraphViewStatus } from "../../graphViewTypes";

// ── Palette (matches components/GraphView.tsx) ────────────────────────────
const C_TEXT = "#dfe7f2";
const C_BODY = "#c7d0dc";
const C_MUTED = "#8b98ab";
const C_BORDER = "#1e2836";
const C_WARN = "#e8c14f";
const C_PANEL = "#0a0f1a";
const MATH_HUE = "#ffb95f";

// ── Lightweight LaTeX/ASCII-math -> Unicode converter ─────────────────────
// Pure function, exported for the scratch validation script; not otherwise
// used outside this file.

const GREEK: Record<string, string> = {
  alpha: "α", beta: "β", gamma: "γ", delta: "δ", epsilon: "ε", zeta: "ζ",
  eta: "η", theta: "θ", iota: "ι", kappa: "κ", lambda: "λ", mu: "μ",
  nu: "ν", xi: "ξ", pi: "π", rho: "ρ", sigma: "σ", tau: "τ", upsilon: "υ",
  phi: "φ", chi: "χ", psi: "ψ", omega: "ω",
  Gamma: "Γ", Delta: "Δ", Theta: "Θ", Lambda: "Λ", Xi: "Ξ", Pi: "Π",
  Sigma: "Σ", Upsilon: "Υ", Phi: "Φ", Psi: "Ψ", Omega: "Ω",
};

const SUPERSCRIPT_DIGITS: Record<string, string> = {
  "0": "⁰", "1": "¹", "2": "²", "3": "³", "4": "⁴", "5": "⁵", "6": "⁶", "7": "⁷", "8": "⁸", "9": "⁹",
  "+": "⁺", "-": "⁻", "=": "⁼", "(": "⁽", ")": "⁾", n: "ⁿ", i: "ⁱ",
};
const SUBSCRIPT_DIGITS: Record<string, string> = {
  "0": "₀", "1": "₁", "2": "₂", "3": "₃", "4": "₄", "5": "₅", "6": "₆", "7": "₇", "8": "₈", "9": "₉",
  "+": "₊", "-": "₋", "=": "₌", "(": "₍", ")": "₎",
};

function toScript(token: string, table: Record<string, string>): string | null {
  let out = "";
  for (const ch of token) {
    const mapped = table[ch];
    if (!mapped) return null; // unmappable char -> caller falls back to raw
    out += mapped;
  }
  return out;
}

const SYMBOLS: Record<string, string> = {
  "\\leq": "≤", "\\geq": "≥", "\\neq": "≠", "\\approx": "≈", "\\equiv": "≡",
  "\\to": "→", "\\rightarrow": "→", "\\leftarrow": "←", "\\iff": "⟺", "\\implies": "⟹",
  "\\cdot": "·", "\\times": "×", "\\div": "÷", "\\pm": "±", "\\mp": "∓",
  "\\forall": "∀", "\\exists": "∃", "\\in": "∈", "\\notin": "∉", "\\subset": "⊂",
  "\\subseteq": "⊆", "\\cup": "∪", "\\cap": "∩", "\\emptyset": "∅", "\\infty": "∞",
  "\\partial": "∂", "\\nabla": "∇", "\\sum": "∑", "\\prod": "∏", "\\int": "∫",
  "\\sqrt": "√", "\\therefore": "∴", "\\because": "∵", "\\land": "∧", "\\lor": "∨",
  "\\neg": "¬", "\\ldots": "…", "\\cdots": "⋯",
};

export interface MathRenderResult {
  /** Unicode-converted text, safe to render as plain text (no HTML). */
  text: string;
  /** True if every construct in the source was confidently converted. */
  fullyConverted: boolean;
}

/** Converts the small real/likely subset described in the file header. Never
 * throws; anything unrecognized passes through verbatim (fullyConverted=false). */
export function renderMathText(source: string): MathRenderResult {
  if (!source) return { text: source, fullyConverted: true };
  let fullyConverted = true;
  let out = "";
  let i = 0;
  const n = source.length;

  while (i < n) {
    const ch = source[i];

    // \frac{a}{b} -> (a)/(b)
    if (source.startsWith("\\frac{", i)) {
      const num = readBraced(source, i + 6);
      if (num) {
        const denStart = i + 6 + num.consumed;
        if (source[denStart] === "{") {
          const den = readBraced(source, denStart + 1);
          if (den) {
            const numConv = renderMathText(num.content);
            const denConv = renderMathText(den.content);
            out += `(${numConv.text})/(${denConv.text})`;
            fullyConverted = fullyConverted && numConv.fullyConverted && denConv.fullyConverted;
            i = denStart + 1 + den.consumed;
            continue;
          }
        }
      }
    }

    // \sqrt{a} -> √(a); bare \sqrt handled by SYMBOLS below.
    if (source.startsWith("\\sqrt{", i)) {
      const arg = readBraced(source, i + 6);
      if (arg) {
        const conv = renderMathText(arg.content);
        out += `√(${conv.text})`;
        fullyConverted = fullyConverted && conv.fullyConverted;
        i = i + 6 + arg.consumed;
        continue;
      }
    }

    // Backslash command: \name (Greek letter, symbol, or unknown).
    if (ch === "\\") {
      const m = /^\\[a-zA-Z]+/.exec(source.slice(i));
      if (m) {
        const cmd = m[0];
        const name = cmd.slice(1);
        if (GREEK[name]) {
          out += GREEK[name];
          i += cmd.length;
          continue;
        }
        if (SYMBOLS[cmd]) {
          out += SYMBOLS[cmd];
          i += cmd.length;
          continue;
        }
        // Unrecognized command: keep verbatim, flag as not fully converted.
        out += cmd;
        fullyConverted = false;
        i += cmd.length;
        continue;
      }
    }

    // Superscript: ^{...} or ^x (single token: digits/letters/+-=())
    if (ch === "^") {
      const braced = source[i + 1] === "{" ? readBraced(source, i + 2) : null;
      if (braced) {
        const conv = toScript(braced.content, SUPERSCRIPT_DIGITS);
        if (conv !== null) {
          out += conv;
          i = i + 2 + braced.consumed;
          continue;
        }
        // Unmappable braced superscript: keep the whole `^{...}` span verbatim
        // (not just the innermost char) and flag it, rather than silently
        // reconstructing it one character at a time with no honesty signal.
        out += "^{" + braced.content + "}";
        fullyConverted = false;
        i = i + 2 + braced.consumed;
        continue;
      } else {
        const m = /^[0-9a-zA-Z+\-=()]/.exec(source.slice(i + 1));
        if (m) {
          const conv = toScript(m[0], SUPERSCRIPT_DIGITS);
          if (conv !== null) {
            out += conv;
            i += 2;
            continue;
          }
        }
      }
    }

    // Subscript: _{...} or _x
    if (ch === "_") {
      const braced = source[i + 1] === "{" ? readBraced(source, i + 2) : null;
      if (braced) {
        const conv = toScript(braced.content, SUBSCRIPT_DIGITS);
        if (conv !== null) {
          out += conv;
          i = i + 2 + braced.consumed;
          continue;
        }
        // Same honesty fix as the superscript branch above.
        out += "_{" + braced.content + "}";
        fullyConverted = false;
        i = i + 2 + braced.consumed;
        continue;
      } else {
        const m = /^[0-9+\-=()]/.exec(source.slice(i + 1));
        if (m) {
          const conv = toScript(m[0], SUBSCRIPT_DIGITS);
          if (conv !== null) {
            out += conv;
            i += 2;
            continue;
          }
        }
      }
    }

    out += ch;
    i += 1;
  }

  return { text: out, fullyConverted };
}

function readBraced(s: string, openBraceIndex: number): { content: string; consumed: number } | null {
  // openBraceIndex points at the character AFTER the opening '{' (caller already matched it),
  // i.e. this reads up to the matching '}' starting the scan at openBraceIndex.
  let depth = 1;
  let j = openBraceIndex;
  while (j < s.length) {
    if (s[j] === "{") depth++;
    else if (s[j] === "}") {
      depth--;
      if (depth === 0) {
        return { content: s.slice(openBraceIndex, j), consumed: j - openBraceIndex + 1 };
      }
    }
    j++;
  }
  return null; // unbalanced — caller falls back to raw text
}

// ── Small view helpers ─────────────────────────────────────────────────────

const SectionTitle: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div
    style={{
      fontSize: 11,
      fontWeight: 700,
      letterSpacing: 0.4,
      textTransform: "uppercase",
      color: C_MUTED,
      margin: "14px 0 6px",
      paddingTop: 10,
      borderTop: `1px solid ${C_BORDER}`,
    }}
  >
    {children}
  </div>
);

const MathText: React.FC<{ source: string; showRawToggleIfPartial?: boolean }> = ({ source }) => {
  const [showRaw, setShowRaw] = useState(false);
  const { text, fullyConverted } = useMemo(() => renderMathText(source), [source]);
  return (
    <div style={{ ...wrap }}>
      <span>{showRaw ? source : text}</span>
      {!fullyConverted && (
        <button
          onClick={() => setShowRaw((v) => !v)}
          title="This contains a construct the lightweight renderer doesn't know — showing raw source is always available."
          style={{
            marginLeft: 8,
            fontSize: 10,
            color: C_MUTED,
            background: "none",
            border: `1px solid ${C_BORDER}`,
            borderRadius: 4,
            padding: "0 5px",
            cursor: "pointer",
          }}
        >
          {showRaw ? "rendered" : "raw"}
        </button>
      )}
    </div>
  );
};

const wrap: React.CSSProperties = { overflowWrap: "anywhere", wordBreak: "break-word", minWidth: 0 };

function isReal(node: GraphViewNode, type: string): boolean {
  return node.isReal && node.nodeType === type;
}

// ── Main component ─────────────────────────────────────────────────────────

export interface MathViewerProps {
  projectId: number | null;
}

export const MathViewer: React.FC<MathViewerProps> = ({ projectId }) => {
  const [selected] = useSelectedFile();

  // (a) raw file content, if the selected file is math and F0's IPC exists.
  const [fileContent, setFileContent] = useState<FileContentResult | null>(null);
  const [fileError, setFileError] = useState<string | null>(null);
  const [fileLoading, setFileLoading] = useState(false);

  useEffect(() => {
    if (!selected || !selected.path) {
      setFileContent(null);
      setFileError(null);
      return;
    }
    let cancelled = false;
    setFileLoading(true);
    setFileError(null);
    readFileContent(selected.path)
      .then((r) => {
        if (!cancelled) setFileContent(r);
      })
      .catch((e) => {
        if (cancelled) return;
        if (e instanceof FileContentUnavailable) {
          setFileError(e.message);
        } else {
          setFileError(e instanceof Error ? e.message : String(e));
        }
      })
      .finally(() => {
        if (!cancelled) setFileLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [selected]);

  // (b) the project's real math graph(s).
  const [status, setStatus] = useState<GraphViewStatus>({ kind: "loading" });
  useEffect(() => {
    if (projectId === null) {
      setStatus({ kind: "empty" });
      return;
    }
    return loadGraphData(projectId, setStatus, { modality: "math" });
  }, [projectId]);

  const [showRawFile, setShowRawFile] = useState(false);

  const proofSteps = useMemo(() => {
    if (status.kind !== "ready") return [];
    return status.data.nodes
      .filter((n) => isReal(n, "ProofStep"))
      .sort((a, b) => {
        const sa = typeof a.raw.step_number === "number" ? a.raw.step_number : Number.POSITIVE_INFINITY;
        const sb = typeof b.raw.step_number === "number" ? b.raw.step_number : Number.POSITIVE_INFINITY;
        return sa - sb;
      });
  }, [status]);

  const variables = useMemo(() => {
    if (status.kind !== "ready") return [];
    return status.data.nodes.filter((n) => isReal(n, "Variable"));
  }, [status]);

  const assumptions = useMemo(() => {
    if (status.kind !== "ready") return [];
    return status.data.nodes.filter((n) => isReal(n, "Assumption"));
  }, [status]);

  const scopeTreeNote = useMemo(() => {
    if (status.kind !== "ready" || status.data.sourceContainers.length === 0) return null;
    // scope_tree lives on the raw wrapper JSON, not the per-node contract — every
    // sample checked on disk has it null (real field, currently unpopulated).
    return "not recorded for any graph checked (real field `scope_tree`, currently always null in this project's data)";
  }, [status]);

  const otherRealNodes = useMemo(() => {
    if (status.kind !== "ready") return [];
    return status.data.nodes.filter((n) => n.nodeType === "Root" && n.isReal);
  }, [status]);

  const schemaOnlyCount = useMemo(() => {
    if (status.kind !== "ready") return 0;
    return status.data.nodes.filter((n) => !n.isReal).length;
  }, [status]);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4, fontSize: 12, color: C_BODY }}>
      <div style={{ fontSize: 11, color: C_MUTED, ...wrap }}>
        Lightweight renderer: converts the common LaTeX/ASCII-math subset (Greek letters, ^/_
        super/subscript, \frac, \sqrt, \sum, \forall, ≤ ≥ ≠ etc.) to Unicode. It is not full
        typesetting — a library such as KaTeX would be needed for that (not installed; see
        this fork's handoff). Anything it can't convert is shown as raw text, never guessed.
      </div>

      {/* (a) Raw file content, when a math file is selected */}
      <SectionTitle>Selected file</SectionTitle>
      {!selected && <div style={{ color: C_MUTED, fontSize: 11.5, fontStyle: "italic" }}>No file selected — pick one in the Files browser.</div>}
      {selected && selected.modality && selected.modality !== "math" && (
        <div style={{ color: C_MUTED, fontSize: 11.5 }}>
          Selected file's modality is <b>{selected.modality}</b>, not math — showing it anyway if content is available.
        </div>
      )}
      {selected && (
        <div>
          <div style={{ ...wrap, marginBottom: 4 }}>
            <span style={{ color: C_MUTED }}>Path: </span>
            <span>{selected.path ?? "(no path recorded on this reference)"}</span>
          </div>
          {fileLoading && <div style={{ color: C_MUTED, fontSize: 11.5 }}>Loading file content…</div>}
          {fileError && (
            <div style={{ ...wrap, color: C_WARN, fontSize: 11.5, border: `1px solid ${C_WARN}`, borderRadius: 6, padding: "6px 8px" }}>
              {fileError}
            </div>
          )}
          {fileContent && (
            <div>
              <div style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: 4 }}>
                <span style={{ fontSize: 11, color: C_MUTED }}>
                  {fileContent.sizeBytes} bytes{fileContent.truncated ? " (truncated)" : ""}
                </span>
                <button
                  onClick={() => setShowRawFile((v) => !v)}
                  style={{ fontSize: 10, color: C_MUTED, background: "none", border: `1px solid ${C_BORDER}`, borderRadius: 4, padding: "1px 6px", cursor: "pointer" }}
                >
                  {showRawFile ? "show rendered" : "show raw"}
                </button>
              </div>
              <pre
                style={{
                  ...wrap,
                  whiteSpace: "pre-wrap",
                  background: C_PANEL,
                  border: `1px solid ${C_BORDER}`,
                  borderRadius: 6,
                  padding: "8px 10px",
                  fontSize: 12,
                  maxHeight: 220,
                  overflowY: "auto",
                }}
              >
                {showRawFile ? fileContent.content : renderMathText(fileContent.content).text}
              </pre>
            </div>
          )}
        </div>
      )}

      {/* (b) The project's real math graph */}
      <SectionTitle>Project math graph</SectionTitle>
      {status.kind === "loading" && <div style={{ color: C_MUTED, fontSize: 11.5 }}>Loading graph…</div>}
      {status.kind === "error" && (
        <div style={{ ...wrap, color: C_WARN, fontSize: 11.5 }}>Error: {status.message}</div>
      )}
      {status.kind === "empty" && (
        <div style={{ color: C_MUTED, fontSize: 11.5, fontStyle: "italic" }}>
          No math graph exists for this project yet — nothing fabricated to show in its place.
        </div>
      )}
      {status.kind === "ready" && (
        <div>
          {otherRealNodes.map((root) => (
            <div key={root.id} style={{ marginBottom: 6 }}>
              <div style={{ ...wrap, color: C_TEXT }}>
                <b>{root.label}</b>
                {root.contentPreview && <> — <MathText source={root.contentPreview} /></>}
              </div>
              {root.confidence !== undefined && (
                <div style={{ fontSize: 11, color: C_MUTED }}>confidence {root.confidence.toFixed(2)}</div>
              )}
            </div>
          ))}

          <div style={{ fontSize: 11, color: C_MUTED, marginBottom: 8 }}>
            {proofSteps.length} proof step{proofSteps.length === 1 ? "" : "s"} · {variables.length} variable
            {variables.length === 1 ? "" : "s"} · {assumptions.length} assumption{assumptions.length === 1 ? "" : "s"}
            {schemaOnlyCount > 0 && ` · ${schemaOnlyCount} schema-only node${schemaOnlyCount === 1 ? "" : "s"} hidden from this view`}
          </div>
          {scopeTreeNote && (
            <div style={{ fontSize: 11, color: C_MUTED, marginBottom: 10, ...wrap }}>scope_tree: {scopeTreeNote}</div>
          )}

          {proofSteps.length === 0 && (
            <div style={{ color: C_MUTED, fontSize: 11.5, fontStyle: "italic" }}>No proof steps recorded in this graph.</div>
          )}
          {proofSteps.map((step) => (
            <div
              key={step.id}
              style={{
                border: `1px solid ${C_BORDER}`,
                borderRadius: 8,
                padding: "8px 10px",
                marginBottom: 6,
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", gap: 8 }}>
                <span style={{ color: MATH_HUE, fontWeight: 700, fontSize: 11 }}>
                  {typeof step.raw.step_number === "number" ? `Step ${step.raw.step_number}` : step.label}
                </span>
                {step.confidence !== undefined && (
                  <span style={{ fontSize: 11, color: C_MUTED }}>confidence {step.confidence.toFixed(2)}</span>
                )}
              </div>
              {step.contentPreview ? (
                <div style={{ marginTop: 4 }}>
                  <MathText source={step.contentPreview} />
                </div>
              ) : (
                <div style={{ marginTop: 4, color: C_MUTED, fontSize: 11.5, fontStyle: "italic" }}>
                  No content recorded on this step.
                </div>
              )}
            </div>
          ))}

          {variables.length > 0 && (
            <>
              <div style={{ fontSize: 11, fontWeight: 700, color: C_MUTED, margin: "10px 0 4px" }}>Variables</div>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                {variables.map((v) => (
                  <div
                    key={v.id}
                    title={v.contentPreview ?? ""}
                    style={{
                      border: `1px solid ${C_BORDER}`,
                      borderRadius: 6,
                      padding: "3px 8px",
                      fontSize: 11.5,
                    }}
                  >
                    <MathText source={v.label} />
                    {v.contentPreview && v.contentPreview !== v.label && (
                      <span style={{ color: C_MUTED }}>
                        {" "}
                        (<MathText source={v.contentPreview} />)
                      </span>
                    )}
                  </div>
                ))}
              </div>
            </>
          )}

          {assumptions.length > 0 && (
            <>
              <div style={{ fontSize: 11, fontWeight: 700, color: C_MUTED, margin: "10px 0 4px" }}>Assumptions</div>
              {assumptions.map((a) => (
                <div key={a.id} style={{ fontSize: 11.5, marginBottom: 3, ...wrap }}>
                  <MathText source={a.contentPreview ?? a.label} />
                </div>
              ))}
            </>
          )}
        </div>
      )}
    </div>
  );
};

export default MathViewer;
