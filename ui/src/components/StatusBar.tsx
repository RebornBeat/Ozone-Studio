/**
 * StatusBar Component - BOTTOM status bar
 *
 * The STATUS BAR is the soul of Ozone Studio's collective vision.
 * It prominently displays COLLECTIVE CONTRIBUTIONS to remind users
 * that every interaction builds the shared knowledge ecosystem.
 *
 * peerCount/p2pEnabled are real (GET /health → NetworkManager::get_status).
 * totalContributions/methodologiesShared/blueprintsShared/findingsShared/
 * myMethodologiesShared/myBlueprintsShared/myFindingsShared/zseiContainers/
 * zseiDepth are still placeholders awaiting a real backend collective-stats
 * system/endpoint — kept here intentionally as the intended future surface,
 * always 0 until that lands.
 *
 * 2026-10-13: these blocks were briefly removed, then restored, because the
 * real backend tracking is planned future work, not an abandoned feature —
 * the placeholder surface is meant to stay visible. What WAS wrong, and is
 * fixed now: the "My Contributions" breakdown used to derive its three
 * sub-values via `Math.floor(myContributions * 0.4/0.35/0.25)` — an
 * arbitrary ratio of one placeholder presented as three distinct tracked
 * numbers. Every value here is now independently sourced (its own field,
 * never derived from another), matching the pattern the "Collective
 * Contributions" block above it already used correctly. When the real
 * endpoint lands, each field gets wired to its own real value.
 */

import React, { useEffect, useState } from 'react';
import { useOzoneStore } from '../services/store';
import { SystemStats } from '../App';

export function StatusBar() {
  const {
    isConnected,
    systemStats,
    consciousnessEnabled,
    quotaStatus,
    fetchQuota,
    ledgerSummary,
    fetchLedgerSummary,
  } = useOzoneStore();

  // OpenRouter free-model quota (GET /quota) + per-model outcome ledger
  // (GET /model-ledger-summary). No existing generic polling utility was
  // found in this file or store.ts, so this is a self-contained poll — note
  // for whoever next touches this: if a shared polling hook exists elsewhere
  // (e.g. App.tsx/main.js), this should be merged into it rather than
  // duplicated further.
  useEffect(() => {
    fetchQuota();
    fetchLedgerSummary();
    const id = setInterval(() => {
      fetchQuota();
      fetchLedgerSummary();
    }, 60_000);
    return () => clearInterval(id);
  }, [fetchQuota, fetchLedgerSummary]);

  // Format large numbers with commas
  const formatNumber = (num: number): string => {
    return num.toLocaleString();
  };

  // Get connection status display
  const getConnectionDisplay = () => {
    if (!isConnected) {
      return {
        color: '#f87171', // red
        text: 'Offline',
        icon: '○',
      };
    }
    if (systemStats.p2pEnabled && systemStats.peerCount > 0) {
      return {
        color: '#4ade80', // green
        text: 'Connected',
        icon: '●',
      };
    }
    return {
      color: '#fbbf24', // yellow
      text: 'Local Only',
      icon: '◐',
    };
  };

  const connection = getConnectionDisplay();

  // Calculate total contributions
  const totalContributions = systemStats.totalContributions ||
    (systemStats.methodologiesShared + systemStats.blueprintsShared + systemStats.findingsShared);

  return (
    <footer className="status-bar">
      {/* Connection Status */}
      <div className="status-item status-connection" title={isConnected ? 'Backend connected' : 'Backend not running'}>
        <span 
          className="status-indicator"
          style={{ color: connection.color }}
        >
          {connection.icon}
        </span>
        <span className="status-text">{connection.text}</span>
      </div>

      {/* Divider */}
      <div className="status-divider" />

      {/* P2P Peers */}
      <div className="status-item status-peers" title="Connected P2P peers">
        <span className="status-icon">👥</span>
        <span className="status-value">{formatNumber(systemStats.peerCount)}</span>
        <span className="status-label">Peers</span>
      </div>

      {/* Divider */}
      <div className="status-divider" />

      {/* OpenRouter free-model quota (GET /quota) — never a fabricated
          number: shows "quota: ?" until the host has a real reading. */}
      <div
        className="status-item status-quota"
        title={
          quotaStatus?.known
            ? `OpenRouter free-model quota, ${quotaStatus.tier ?? "unknown tier"}`
            : "OpenRouter quota not read yet"
        }
      >
        <span className="status-icon">🔑</span>
        <span className="status-value">
          {quotaStatus?.known
            ? `${formatNumber(quotaStatus.remaining ?? 0)}/${formatNumber(quotaStatus.limit ?? 0)}`
            : "?"}
        </span>
        <span className="status-label">Free Quota</span>
      </div>

      {/* Divider */}
      <div className="status-divider" />

      {/* COLLECTIVE CONTRIBUTIONS - THE MAIN FOCUS */}
      <div className="status-item status-contributions primary" title="Total collective contributions to the ecosystem">
        <span className="status-icon">✨</span>
        <span className="status-value highlight">{formatNumber(totalContributions)}</span>
        <span className="status-label">Collective Contributions</span>

        {/* Contribution breakdown tooltip/dropdown */}
        <div className="contribution-breakdown">
          <div className="breakdown-item">
            <span className="breakdown-icon">📚</span>
            <span className="breakdown-value">{formatNumber(systemStats.methodologiesShared)}</span>
            <span className="breakdown-label">Methodologies</span>
          </div>
          <div className="breakdown-item">
            <span className="breakdown-icon">📋</span>
            <span className="breakdown-value">{formatNumber(systemStats.blueprintsShared)}</span>
            <span className="breakdown-label">Blueprints</span>
          </div>
          <div className="breakdown-item">
            <span className="breakdown-icon">💡</span>
            <span className="breakdown-value">{formatNumber(systemStats.findingsShared)}</span>
            <span className="breakdown-label">Findings</span>
          </div>
        </div>
      </div>

      {/* My Contributions */}
      <div className="status-item status-my-contributions" title="Your contributions to the ecosystem">
        <span className="status-icon">🎯</span>
        <span className="status-value">{formatNumber(systemStats.myContributions)}</span>
        <span className="status-label">My Contributions</span>

        {/* My contribution breakdown on hover — each value independently
            sourced (own placeholder field), never derived from myContributions
            via an arbitrary ratio. See the file header for why. */}
        <div className="contribution-breakdown">
          <div className="breakdown-title">Your Contributions</div>
          <div className="breakdown-item">
            <span className="breakdown-icon">📚</span>
            <span className="breakdown-value">{formatNumber(systemStats.myMethodologiesShared)}</span>
            <span className="breakdown-label">Methodologies</span>
          </div>
          <div className="breakdown-item">
            <span className="breakdown-icon">📋</span>
            <span className="breakdown-value">{formatNumber(systemStats.myBlueprintsShared)}</span>
            <span className="breakdown-label">Blueprints</span>
          </div>
          <div className="breakdown-item">
            <span className="breakdown-icon">💡</span>
            <span className="breakdown-value">{formatNumber(systemStats.myFindingsShared)}</span>
            <span className="breakdown-label">Findings</span>
          </div>
        </div>
      </div>

      {/* Divider */}
      <div className="status-divider" />

      {/* Per-model outcome ledger (GET /model-ledger-summary) — real
          captured outcomes only: shows "—" until the host answers, and the
          hover breakdown lists each model's counts plus today's attributed
          failure causes. Never a fabricated success rate. */}
      <div
        className="status-item status-ledger"
        title={
          ledgerSummary
            ? "Per-model outcome ledger (all history) — hover for the breakdown"
            : "Model ledger not read yet"
        }
      >
        <span className="status-icon">📓</span>
        <span className="status-value">
          {ledgerSummary
            ? (() => {
                const ok = ledgerSummary.models.reduce((n, m) => n + m.successes, 0);
                const total = ledgerSummary.models.reduce((n, m) => n + m.attempts, 0);
                return `${formatNumber(ok)}/${formatNumber(total)}`;
              })()
            : "—"}
        </span>
        <span className="status-label">Ledger</span>

        {ledgerSummary && (
          <div className="contribution-breakdown">
            <div className="breakdown-title">Model ledger (all history)</div>
            {ledgerSummary.models.map((m) => (
              <div
                className="breakdown-item"
                key={m.model}
                title={`${m.successes} succeeded · ${m.empties} empty · ${m.http_errors} http · ${m.timeouts} timeout · ${m.other_errors} other${
                  m.median_latency_ms != null ? ` · median ${formatNumber(m.median_latency_ms)}ms` : ""
                } · last: ${m.last_outcome}`}
              >
                <span className="breakdown-icon" style={{ fontSize: 9 }}>
                  {m.successes > 0 ? "✓" : "✗"}
                </span>
                <span className="breakdown-label" style={{ maxWidth: 260, overflow: 'hidden', textOverflow: 'ellipsis' }}>
                  {m.model}
                </span>
                <span className="breakdown-value">
                  {m.successes}/{m.attempts}
                </span>
              </div>
            ))}
            {ledgerSummary.causes_today.length > 0 && (
              <>
                <div className="breakdown-title" style={{ marginTop: 4 }}>
                  Attributed failures today (UTC)
                </div>
                {ledgerSummary.causes_today
                  .filter((c) => !c.tag.includes(" "))
                  .map((c) => (
                    <div className="breakdown-item" key={c.tag}>
                      <span className="breakdown-icon" style={{ fontSize: 9 }}>⚑</span>
                      <span className="breakdown-label">{c.tag}</span>
                      <span className="breakdown-value">{formatNumber(c.count)}</span>
                    </div>
                  ))}
              </>
            )}
          </div>
        )}
      </div>

      {/* Divider */}
      <div className="status-divider" />

      {/* ZSEI Status */}
      <div className="status-item status-zsei" title="ZSEI knowledge containers and traversal depth">
        <span className="status-icon">🔍</span>
        <span className="status-value">{formatNumber(systemStats.zseiContainers)}</span>
        <span className="status-label">ZSEI</span>
        <span className="status-sublabel">Depth: {systemStats.zseiDepth}</span>
      </div>

      {/* Divider */}
      <div className="status-divider" />

      {/* Active Tasks */}
      <div className="status-item status-tasks" title="Active pipeline tasks">
        <span className="status-icon">⚡</span>
        <span className="status-value">{systemStats.activeTaskCount}</span>
        <span className="status-label">Tasks</span>
      </div>

      {/* Consciousness Status (if enabled) */}
      {consciousnessEnabled && (
        <>
          <div className="status-divider" />
          <div className="status-item status-consciousness" title="Consciousness system status">
            <span className="status-icon">🧠</span>
            <span className="status-value">{systemStats.consciousnessState || 'Active'}</span>
            <span className="status-label">I-Loop</span>
            {systemStats.iLoopStatus && (
              <span className="status-sublabel">{systemStats.iLoopStatus}</span>
            )}
          </div>
        </>
      )}

      {/* System Resources (right-aligned) */}
      <div className="status-spacer" />
      
      {(systemStats.backendMemoryMb != null || systemStats.backendCpuPercent != null) && (
        <div
          className="status-item status-backend-usage"
          title="Ozone-Studio backend process (real RSS/CPU, found by port — works whether it was launched by this app or manually)"
        >
          <span className="status-icon">🖥️</span>
          <span className="status-value">
            {systemStats.backendMemoryMb != null ? `${systemStats.backendMemoryMb}MB` : '—'}
            {systemStats.backendCpuPercent != null
              ? ` · ${systemStats.backendCpuPercent.toFixed(0)}%`
              : ''}
          </span>
          <span className="status-label">Backend</span>
        </div>
      )}

      <div className="status-item status-memory" title="This UI (Electron) process's own memory — not the backend">
        <span className="status-icon">💾</span>
        <span className="status-value">{Math.round(systemStats.memoryUsage)}%</span>
        <span className="status-label">UI Mem</span>
      </div>

      {/* Uptime */}
      <div className="status-item status-uptime" title="Session uptime">
        <span className="status-icon">⏱️</span>
        <span className="status-value">{formatUptime(systemStats.uptime)}</span>
      </div>
    </footer>
  );
}

// Helper to format uptime
function formatUptime(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  const hours = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  return `${hours}h ${mins}m`;
}
