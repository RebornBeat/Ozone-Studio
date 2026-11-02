/**
 * TranscriptEntryMeta.tsx — per-response meta strip (model badge, stage count,
 * AMT summary, collapsible Thinking log, collapsible attempt Trail + refusals),
 * extracted verbatim from MetaPortion.tsx (2026-11-01) so the global chat and
 * the project chat render identical reasoning indicators. Expand state is local
 * to each entry. Renders nothing for user entries or entries without data.
 */

import React, { useState } from 'react';
import { TranscriptEntry, modelStyle, summarizeTrail } from '../chatRendering';
import { useConsciousnessToggles } from '../hooks/useConsciousnessToggles';

export function TranscriptEntryMeta({ entry }: { entry: TranscriptEntry }) {
  const [thinkingOpen, setThinkingOpen] = useState(false);
  const [trailOpen, setTrailOpen] = useState(false);
  // [consciousness].show_decision_reasoning — hides the Thinking log (reasoning) when false.
  const { showDecisionReasoning } = useConsciousnessToggles();
  const ms = modelStyle(entry.modelUsed);
  return (
    <>
  {entry.role === 'assistant' && (
    entry.modelUsed || entry.stageCount ||
    (entry.attemptTrail && entry.attemptTrail.length > 0) ||
    (entry.refusals && entry.refusals.length > 0)
  ) && (
    <div
      className="message-meta"
      style={{ display: 'flex', gap: 6, alignItems: 'center', marginTop: 5 }}
    >
      {entry.modelUsed && (
        <span
          style={{
            border: `1px solid ${ms.color}55`,
            color: ms.color,
            background: ms.color + '14',
            borderRadius: 999,
            fontSize: 11,
            padding: '2px 8px',
            display: 'inline-flex',
            gap: 5,
            alignItems: 'center',
          }}
        >
          {ms.icon} {entry.modelUsed}
        </span>
      )}
      {entry.stageCount ? (
        <span style={{ fontSize: 11, opacity: 0.5 }}>{entry.stageCount} stages</span>
      ) : null}
      {entry.amtSummary && (
        <span
          title={`AMT: ${entry.amtSummary.total_nodes} nodes, depth ${entry.amtSummary.max_depth}, ${entry.amtSummary.validation_status}`}
          style={{
            fontSize: 11,
            opacity: 0.6,
            border: '1px solid #223046',
            borderRadius: 999,
            padding: '2px 8px',
            display: 'inline-flex',
            gap: 5,
            alignItems: 'center',
          }}
        >
          🌳 {entry.amtSummary.branch_count} branch{entry.amtSummary.branch_count === 1 ? '' : 'es'}
          {' · '}depth {entry.amtSummary.max_depth}
          {entry.amtSummary.validation_status === 'Validated' ? ' · ✓' : ''}
        </span>
      )}
      {showDecisionReasoning && entry.thinkingLog && entry.thinkingLog.length > 0 && (
        <button
          onClick={() => {
            setThinkingOpen(o => !o);
          }}
          style={{
            fontSize: 11,
            opacity: 0.6,
            background: 'none',
            border: '1px solid #223046',
            borderRadius: 999,
            padding: '2px 8px',
            cursor: 'pointer',
            color: 'inherit',
          }}
        >
          {thinkingOpen ? '▾' : '▸'} Thinking ({entry.thinkingLog.length})
        </button>
      )}
      {((entry.attemptTrail && entry.attemptTrail.length > 0) ||
        (entry.refusals && entry.refusals.length > 0)) && (
        <button
          onClick={() => {
            setTrailOpen(o => !o);
          }}
          style={{
            fontSize: 11,
            opacity: 0.6,
            background: 'none',
            border: '1px solid #223046',
            borderRadius: 999,
            padding: '2px 8px',
            cursor: 'pointer',
            color: 'inherit',
          }}
          title="Fallback-walk attempt trail and any paid/unknown-model refusals for this run"
        >
          {trailOpen ? '▾' : '▸'} Trail (
          {(entry.attemptTrail?.length ?? 0) + (entry.refusals?.length ?? 0)})
        </button>
      )}
    </div>
  )}
  {showDecisionReasoning && entry.thinkingLog && entry.thinkingLog.length > 0 && thinkingOpen && (
    <div
      className="thinking-log"
      style={{
        marginTop: 8,
        display: 'flex',
        flexDirection: 'column',
        gap: 6,
      }}
    >
      {entry.thinkingLog.map((t, i) => (
        <div
          key={i}
          style={{
            border: '1px solid #223046',
            borderRadius: 8,
            padding: '8px 10px',
            fontSize: 12,
            background: 'rgba(255,255,255,0.02)',
          }}
        >
          <div
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              gap: 8,
              opacity: 0.7,
              marginBottom: 4,
              fontSize: 11,
            }}
          >
            <span>{t.stage}</span>
            <span>
              {t.tokens_used != null ? `${t.tokens_used} tok` : ''}
              {t.eval_tokens_per_sec != null
                ? ` · ${t.eval_tokens_per_sec.toFixed(1)} tok/s`
                : ''}
            </span>
          </div>
          <div style={{ whiteSpace: 'pre-wrap', opacity: 0.85, overflowWrap: 'break-word', wordBreak: 'break-word' }}>
            {t.raw_response}
          </div>
        </div>
      ))}
    </div>
  )}
  {((entry.attemptTrail && entry.attemptTrail.length > 0) ||
    (entry.refusals && entry.refusals.length > 0)) &&
    trailOpen && (
    <div
      className="attempt-trail"
      style={{
        marginTop: 8,
        display: 'flex',
        flexDirection: 'column',
        gap: 6,
        fontSize: 12,
      }}
    >
      {entry.attemptTrail && entry.attemptTrail.length > 0 && (
        <div
          style={{
            border: '1px solid #223046',
            borderRadius: 8,
            padding: '8px 10px',
            background: 'rgba(255,255,255,0.02)',
          }}
        >
          <div style={{ opacity: 0.7, marginBottom: 4, fontSize: 11 }}>
            Attempt trail ({entry.attemptTrail.length})
          </div>
          {(() => {
            const trail = entry.attemptTrail ?? [];
            const ts = summarizeTrail(trail);
            const causeText = Object.entries(ts.causes)
              .map(([c, n]) => `${c} ${n}`)
              .join(', ');
            const bits: string[] = [];
            if (ts.succeeded > 0) bits.push(`${ts.succeeded} answered`);
            if (ts.failed > 0) bits.push(`${ts.failed} failed`);
            if (ts.skipped > 0) bits.push(`${ts.skipped} skipped (quota)`);
            if (ts.sameModelRetries > 0) bits.push(`${ts.sameModelRetries} same-model retr${ts.sameModelRetries === 1 ? 'y' : 'ies'}`);
            if (ts.windowCapped > 0) bits.push(`${ts.windowCapped} window-capped`);
            if (ts.timeouts > 0) bits.push(`${ts.timeouts} timed out`);
            if (ts.recoveredByWalk) bits.push('recovered by fallback walk');
            return (
              <div
                style={{
                  fontSize: 11,
                  opacity: 0.75,
                  marginBottom: 6,
                  paddingBottom: 6,
                  borderBottom: '1px solid #223046',
                }}
                title="Computed from the trail lines below — counts only what the backend wrote"
              >
                {bits.length > 0 ? bits.join(' · ') : `${ts.attempts} attempt${ts.attempts === 1 ? '' : 's'}`}
                {causeText && ` — causes: ${causeText}`}
                {ts.unparsable > 0 && ` · ${ts.unparsable} line${ts.unparsable === 1 ? '' : 's'} not summarized (shown raw)`}
              </div>
            );
          })()}
          {entry.attemptTrail.map((line, i) => (
            <div key={i} style={{ whiteSpace: 'pre-wrap', opacity: 0.85, overflowWrap: 'break-word', wordBreak: 'break-word' }}>
              {line}
            </div>
          ))}
        </div>
      )}
      {entry.refusals && entry.refusals.length > 0 && (
        <div
          style={{
            border: '1px solid #5a2a2a',
            borderRadius: 8,
            padding: '8px 10px',
            background: 'rgba(255,100,100,0.04)',
          }}
        >
          <div style={{ opacity: 0.8, marginBottom: 4, fontSize: 11, color: '#f87171' }}>
            ⛔ Refused ({entry.refusals.length})
          </div>
          {entry.refusals.map((line, i) => (
            <div key={i} style={{ whiteSpace: 'pre-wrap', opacity: 0.85, overflowWrap: 'break-word', wordBreak: 'break-word' }}>
              {line}
            </div>
          ))}
        </div>
      )}
    </div>
  )}
    </>
  );
}

export default TranscriptEntryMeta;
