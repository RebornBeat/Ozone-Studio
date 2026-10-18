/**
 * chatRendering.tsx — chat-rendering pieces extracted out of MetaPortion.tsx
 * (2026-10-18) so they're defined once instead of inline. MetaPortion is the
 * only consumer: a separate embedded project-scoped chat panel was built and
 * then removed the same day (2026-10-18) as redundant with MetaPortion's own
 * existing global/workspace context switcher — this file stayed because the
 * extraction itself is a real simplification independent of that.
 */

import React from 'react';

export interface ThinkingEntry {
  stage: string;
  raw_response: string;
  tokens_used?: number;
  model_used?: string;
  eval_tokens_per_sec?: number;
  prompt_eval_tokens_per_sec?: number;
  load_time_ms?: number;
}

/** Real, aggregate AMT structure numbers — forwarded verbatim from
 * OrchestrateResponse.amt_summary (src/orchestrator/mod.rs's AMTSummary).
 * Deliberately just these 4 fields: today's backend doesn't expose
 * per-branch names/relationships/confidence to the client, only these
 * aggregate counts — do not fabricate a richer tree than the data
 * actually supports. */
export interface AMTSummary {
  total_nodes: number;
  branch_count: number;
  max_depth: number;
  validation_status: string;
}

export interface TranscriptEntry {
  id: number;
  role: 'user' | 'assistant';
  content: string;
  timestamp: number;
  emotion?: string;
  /** Which model produced this response — real per-call data forwarded from
   * OrchestrateResponse.model_used, not guessed. */
  modelUsed?: string;
  stageCount?: number;
  /** Full "thinking cycle" — one entry per real LLM call this run made
   * (AMT-building passes, blueprint drafting, zero-shot simulation, step
   * execution), forwarded from OrchestrateResponse.thinking_log. Empty/
   * absent for user messages and older responses from before this existed. */
  thinkingLog?: ThinkingEntry[];
  /** Real AMT structure stats for this response, when the run built one
   * (absent for e.g. a clarification-only response with no AMT). */
  amtSummary?: AMTSummary;
  /** Per-attempt fallback-walk trail (model, outcome, cause, latency, next
   * step), forwarded verbatim from OrchestrateResponse.attempt_trail. Always
   * an array (possibly empty) on any response carrying these fields — older
   * responses simply omit the key. Present on BOTH success and failure. */
  attemptTrail?: string[];
  /** Paid/unknown model-override refusals for this run, forwarded verbatim
   * from OrchestrateResponse.refusals. Present on BOTH success and failure. */
  refusals?: string[];
}

// Per-model accent so a model switch mid-conversation is visible at a
// glance, not just a faint text label. Matched by substring against
// whatever model_used/model_identifier string the backend reports.
export const MODEL_STYLES: Record<string, { color: string; icon: string }> = {
  bitnet: { color: '#22d3ee', icon: '⚡' },
  anthropic: { color: '#fb923c', icon: '◈' },
  claude: { color: '#fb923c', icon: '◈' },
  openrouter: { color: '#a78bfa', icon: '◇' },
  openai: { color: '#10b981', icon: '◆' },
  gpt: { color: '#10b981', icon: '◆' },
  zcode: { color: '#4ade80', icon: '❖' },
};
export const DEFAULT_MODEL_STYLE = { color: '#6ec3ff', icon: '●' };
export function modelStyle(model?: string): { color: string; icon: string } {
  if (!model) return DEFAULT_MODEL_STYLE;
  const key = Object.keys(MODEL_STYLES).find((k) =>
    model.toLowerCase().includes(k),
  );
  return key ? MODEL_STYLES[key] : DEFAULT_MODEL_STYLE;
}

// Client-side summary over one run's attempt_trail lines. The line grammar
// is the backend's own (src/orchestrator/mod.rs note_attempt):
//   "{model} {kind}[ [{cause}]] {latency}ms -> {step}"
// with kinds success|empty|http_error|timeout|other_error|skipped and cause
// tags ozone-studio|provider|model|local-runtime|unknown. Only strings the
// backend actually emits are recognized — anything unparsable still shows in
// the raw trail below, it just is not counted, and `unparsable` says so.
export interface TrailSummary {
  attempts: number;
  succeeded: number;
  failed: number;
  skipped: number;
  causes: Record<string, number>;
  sameModelRetries: number;
  windowCapped: number;
  timeouts: number;
  /** A success line that came after at least one failure: the fallback walk
   * recovered the call. */
  recoveredByWalk: boolean;
  unparsable: number;
}

export function summarizeTrail(lines: string[]): TrailSummary {
  const s: TrailSummary = {
    attempts: 0,
    succeeded: 0,
    failed: 0,
    skipped: 0,
    causes: {},
    sameModelRetries: 0,
    windowCapped: 0,
    timeouts: 0,
    recoveredByWalk: false,
    unparsable: 0,
  };
  let seenFailure = false;
  for (const line of lines) {
    const kindMatch = line.match(
      /\b(success|empty|http_error|timeout|other_error|skipped)\b/,
    );
    if (!kindMatch) {
      s.unparsable += 1;
      continue;
    }
    s.attempts += 1;
    const kind = kindMatch[1];
    const causeMatch = line.match(
      /\[(ozone-studio|provider|model|local-runtime|unknown)\]/,
    );
    if (causeMatch) {
      s.causes[causeMatch[1]] = (s.causes[causeMatch[1]] ?? 0) + 1;
    }
    const step = line.includes(' -> ') ? line.split(' -> ')[1] ?? '' : '';
    if (kind === 'success') {
      s.succeeded += 1;
      if (seenFailure) s.recoveredByWalk = true;
    } else if (kind === 'skipped') {
      s.skipped += 1;
    } else {
      s.failed += 1;
      seenFailure = true;
      if (kind === 'timeout') s.timeouts += 1;
      if (step === 'one same-model retry') s.sameModelRetries += 1;
      if (step.startsWith('our max_tokens was exhausted by reasoning')) {
        s.windowCapped += 1;
      }
    }
  }
  return s;
}

// Minimal local markdown-to-React renderer for assistant responses. This
// project's established pattern (see MathViewer.tsx's own local
// ASCII→Unicode converter) is a small local converter over adding an npm
// dependency — no markdown library exists in package.json. Handles the
// real patterns backend responses actually produce: #/##/### headers,
// **bold**, `inline code`, pipe tables, "-"/"*" unordered list items, and
// blank-line-separated paragraphs.
export function renderInlineMarkdown(text: string): React.ReactNode[] {
  const nodes: React.ReactNode[] = [];
  const re = /(\*\*[^*]+\*\*|`[^`]+`)/g;
  let last = 0;
  let m: RegExpExecArray | null;
  let key = 0;
  while ((m = re.exec(text)) !== null) {
    if (m.index > last) nodes.push(text.slice(last, m.index));
    const token = m[0];
    if (token.startsWith('**')) {
      nodes.push(<strong key={key++}>{token.slice(2, -2)}</strong>);
    } else {
      nodes.push(
        <code
          key={key++}
          style={{ background: 'rgba(255,255,255,0.08)', padding: '1px 4px', borderRadius: 4, fontSize: '0.9em' }}
        >
          {token.slice(1, -1)}
        </code>,
      );
    }
    last = m.index + token.length;
  }
  if (last < text.length) nodes.push(text.slice(last));
  return nodes;
}

export function renderMarkdown(text: string): React.ReactNode {
  const lines = text.split('\n');
  const blocks: React.ReactNode[] = [];
  let i = 0;
  let key = 0;
  let paragraph: string[] = [];
  const wrap: React.CSSProperties = { overflowWrap: 'break-word', wordBreak: 'break-word' };

  const flushParagraph = () => {
    if (paragraph.length > 0) {
      blocks.push(
        <p key={key++} style={{ margin: '0 0 8px 0', ...wrap }}>
          {renderInlineMarkdown(paragraph.join(' '))}
        </p>,
      );
      paragraph = [];
    }
  };

  while (i < lines.length) {
    const line = lines[i];
    const trimmed = line.trim();

    if (trimmed === '') {
      flushParagraph();
      i++;
      continue;
    }

    const headerMatch = trimmed.match(/^(#{1,6})\s+(.*)$/);
    if (headerMatch) {
      flushParagraph();
      const level = headerMatch[1].length;
      const Tag = (`h${Math.min(level + 2, 6)}`) as keyof JSX.IntrinsicElements;
      blocks.push(
        <Tag key={key++} style={{ margin: '10px 0 6px 0', fontSize: level <= 2 ? '1.1em' : '1em', ...wrap }}>
          {renderInlineMarkdown(headerMatch[2])}
        </Tag>,
      );
      i++;
      continue;
    }

    // Pipe table: a row containing '|' immediately followed by a
    // ---/:---/| separator row.
    if (trimmed.includes('|') && /^\|?[\s:|-]+\|?[\s:|-]*$/.test(lines[i + 1]?.trim() ?? '') && (lines[i + 1] ?? '').includes('-')) {
      flushParagraph();
      const parseRow = (l: string) => l.trim().replace(/^\||\|$/g, '').split('|').map((c) => c.trim());
      const header = parseRow(line);
      i += 2;
      const rows: string[][] = [];
      while (i < lines.length && lines[i].trim().includes('|')) {
        rows.push(parseRow(lines[i]));
        i++;
      }
      blocks.push(
        <div key={key++} style={{ overflowX: 'auto', margin: '6px 0' }}>
          <table style={{ borderCollapse: 'collapse', fontSize: '0.92em' }}>
            <thead>
              <tr>
                {header.map((h, hi) => (
                  <th key={hi} style={{ border: '1px solid #223046', padding: '4px 8px', textAlign: 'left' }}>
                    {renderInlineMarkdown(h)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {rows.map((r, ri) => (
                <tr key={ri}>
                  {r.map((c, ci) => (
                    <td key={ci} style={{ border: '1px solid #223046', padding: '4px 8px', ...wrap }}>
                      {renderInlineMarkdown(c)}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>,
      );
      continue;
    }

    // Unordered list — consume consecutive "-"/"*" items.
    if (/^[-*]\s+/.test(trimmed)) {
      flushParagraph();
      const items: string[] = [];
      while (i < lines.length && /^[-*]\s+/.test(lines[i].trim())) {
        items.push(lines[i].trim().replace(/^[-*]\s+/, ''));
        i++;
      }
      blocks.push(
        <ul key={key++} style={{ margin: '4px 0 8px 0', paddingLeft: 20, ...wrap }}>
          {items.map((it, ii) => (
            <li key={ii} style={{ marginBottom: 2 }}>
              {renderInlineMarkdown(it)}
            </li>
          ))}
        </ul>,
      );
      continue;
    }

    paragraph.push(line);
    i++;
  }
  flushParagraph();
  return <>{blocks}</>;
}
