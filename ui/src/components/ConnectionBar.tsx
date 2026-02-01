/**
 * ConnectionBar Component - §5.4 of the specification
 * 
 * Shows:
 * - Network connection status
 * - Peer count (P2P network)
 * - Contribution stats
 * - ZSEI traversal depth
 */

import React, { useEffect, useState } from 'react';
import { useOzoneStore } from '../services/store';

interface ConnectionStats {
  networkStatus: 'connected' | 'disconnected' | 'connecting';
  peerCount: number;
  contributions: number;
  zseiDepth: number;
  syncStatus: 'synced' | 'syncing' | 'out_of_sync';
}

interface ConnectionBarProps {
  backendError?: string | null;
}

export function ConnectionBar({ backendError }: ConnectionBarProps) {
  const { isConnected } = useOzoneStore();
  const [stats, setStats] = useState<ConnectionStats>({
    networkStatus: 'connecting',
    peerCount: 0,
    contributions: 0,
    zseiDepth: 0,
    syncStatus: 'syncing',
  });

  // Update status based on connection
  useEffect(() => {
    setStats(prev => ({
      ...prev,
      networkStatus: isConnected ? 'connected' : 'disconnected',
      syncStatus: isConnected ? 'synced' : 'out_of_sync',
    }));
  }, [isConnected]);

  // Periodically fetch connection stats (only if connected)
  useEffect(() => {
    if (!isConnected || !window.ozone) return;
    
    async function fetchStats() {
      try {
        // Query ZSEI for stats
        const result = await window.ozone?.zsei.query({
          type: 'GetStats',
        });
        
        // In real implementation, parse result
        setStats(prev => ({
          ...prev,
          zseiDepth: 3,
        }));
      } catch (err) {
        console.warn('Failed to fetch stats:', err);
      }
    }
    
    fetchStats();
    const interval = setInterval(fetchStats, 30000); // Every 30 seconds
    
    return () => clearInterval(interval);
  }, [isConnected]);

  // Get status indicator color
  const getStatusColor = (status: string) => {
    switch (status) {
      case 'connected':
      case 'synced':
        return '#4ade80'; // Green
      case 'connecting':
      case 'syncing':
        return '#fbbf24'; // Yellow
      case 'disconnected':
      case 'out_of_sync':
        return '#f87171'; // Red
      default:
        return '#9ca3af'; // Gray
    }
  };

  return (
    <header className="connection-bar">
      {/* Network Status */}
      <div className="connection-item" title={backendError || ''}>
        <span
          className="status-indicator"
          style={{ backgroundColor: getStatusColor(stats.networkStatus) }}
        />
        <span className="status-label">
          {stats.networkStatus === 'connected' ? 'Online' : 
           stats.networkStatus === 'connecting' ? 'Connecting...' : 'Offline'}
        </span>
        {backendError && (
          <span className="error-badge" title={backendError}>⚠️</span>
        )}
      </div>

      {/* Backend Status (when disconnected) */}
      {!isConnected && backendError && (
        <div className="connection-item error-item">
          <span className="item-label" style={{ color: '#f87171' }}>
            {backendError}
          </span>
        </div>
      )}

      {/* Peer Count */}
      <div className="connection-item">
        <span className="item-icon">👥</span>
        <span className="item-value">{stats.peerCount}</span>
        <span className="item-label">Peers</span>
      </div>

      {/* Contributions */}
      <div className="connection-item">
        <span className="item-icon">📊</span>
        <span className="item-value">{stats.contributions}</span>
        <span className="item-label">Contributions</span>
      </div>

      {/* ZSEI Depth */}
      <div className="connection-item">
        <span className="item-icon">🔍</span>
        <span className="item-value">{stats.zseiDepth}</span>
        <span className="item-label">ZSEI Depth</span>
      </div>

      {/* Sync Status */}
      <div className="connection-item">
        <span
          className="status-indicator"
          style={{ backgroundColor: getStatusColor(stats.syncStatus) }}
        />
        <span className="status-label">
          {stats.syncStatus === 'synced' ? 'Synced' :
           stats.syncStatus === 'syncing' ? 'Syncing...' : 'Out of Sync'}
        </span>
      </div>

      {/* App Title */}
      <div className="connection-title">
        <span className="title-text">Ozone Studio</span>
        <span className="title-version">v0.3</span>
      </div>
    </header>
  );
}
