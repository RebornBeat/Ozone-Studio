/**
 * StatusBar Component - BOTTOM status bar
 * 
 * The STATUS BAR is the soul of Ozone Studio's collective vision.
 * It prominently displays COLLECTIVE CONTRIBUTIONS to remind users
 * that every interaction builds the shared knowledge ecosystem.
 * 
 * Layout:
 * ┌─────────────────────────────────────────────────────────────────────────┐
 * │ [●] Online │ 👥 42 Peers │ ✨ 12,345 Contributions │ 🔍 ZSEI: 3 │ 💾 │ 
 * └─────────────────────────────────────────────────────────────────────────┘
 */

import React, { useEffect, useState } from 'react';
import { useOzoneStore } from '../services/store';
import { SystemStats } from '../App';

interface ContributionBreakdown {
  methodologies: number;
  blueprints: number;
  findings: number;
}

export function StatusBar() {
  const { 
    isConnected, 
    systemStats,
    consciousnessEnabled,
  } = useOzoneStore();
  
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
        
        {/* My contribution breakdown on hover */}
        <div className="contribution-breakdown">
          <div className="breakdown-title">Your Contributions</div>
          <div className="breakdown-item">
            <span className="breakdown-icon">📚</span>
            <span className="breakdown-value">{formatNumber(Math.floor(systemStats.myContributions * 0.4))}</span>
            <span className="breakdown-label">Methodologies</span>
          </div>
          <div className="breakdown-item">
            <span className="breakdown-icon">📋</span>
            <span className="breakdown-value">{formatNumber(Math.floor(systemStats.myContributions * 0.35))}</span>
            <span className="breakdown-label">Blueprints</span>
          </div>
          <div className="breakdown-item">
            <span className="breakdown-icon">💡</span>
            <span className="breakdown-value">{formatNumber(Math.floor(systemStats.myContributions * 0.25))}</span>
            <span className="breakdown-label">Findings</span>
          </div>
        </div>
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
      
      <div className="status-item status-memory" title="Memory usage">
        <span className="status-icon">💾</span>
        <span className="status-value">{Math.round(systemStats.memoryUsage)}%</span>
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
