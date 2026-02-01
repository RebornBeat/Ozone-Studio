/**
 * Ozone Studio - Main React Application
 * 
 * OZONE STUDIO — Omnidirectional Zero-Shot Neural Engine
 * A Collective AGI Framework with Optional Consciousness
 * 
 * The UI emphasizes:
 * - Collective Contributions (the ecosystem grows together)
 * - AGI-first design with Consciousness as optional feature
 * - Production-ready with graceful offline handling
 */

import React, { useEffect, useState } from 'react';
import { useOzoneStore } from './services/store';
import { MetaPortion } from './components/MetaPortion';
import { ThemeArea } from './components/ThemeArea';
import { StatusBar } from './components/StatusBar';
import './App.css';

// TypeScript declaration for the ozone API exposed by preload
declare global {
  interface Window {
    ozone?: {
      auth: {
        challenge: (publicKey: Uint8Array) => Promise<{ challenge: Uint8Array }>;
        authenticate: (publicKey: Uint8Array, signature: Uint8Array) => Promise<{ success: boolean; sessionToken?: Uint8Array; error?: string }>;
      };
      pipeline: {
        execute: (pipelineId: number, input: object) => Promise<{ success: boolean; taskId: number; output?: object; error?: string }>;
        list: () => Promise<{ pipelines: Array<{ id: number; name: string }> }>;
      };
      zsei: {
        query: (query: object) => Promise<object>;
      };
      task: {
        status: (taskId: number) => Promise<object>;
        list: () => Promise<{ tasks: Array<object> }>;
      };
      config: {
        get: () => Promise<object>;
        set: (updates: object) => Promise<{ success: boolean }>;
      };
      system: {
        getStats: () => Promise<SystemStats>;
      };
      events: {
        onBackendError: (callback: (data: object) => void) => void;
        onConnectionChange: (callback: (data: { connected: boolean }) => void) => void;
        onStatsUpdate: (callback: (data: SystemStats) => void) => void;
      };
    };
  }
}

export interface SystemStats {
  // Connection
  backendConnected: boolean;
  p2pEnabled: boolean;
  peerCount: number;
  
  // Collective Contributions (the focus!)
  totalContributions: number;
  myContributions: number;
  methodologiesShared: number;
  blueprintsShared: number;
  findingsShared: number;
  
  // ZSEI
  zseiContainers: number;
  zseiDepth: number;
  
  // Consciousness (optional feature)
  consciousnessEnabled: boolean;
  consciousnessState?: string;
  iLoopStatus?: string;
  
  // System
  uptime: number;
  memoryUsage: number;
  activeTaskCount: number;
}

/**
 * Main Application Component
 * 
 * Layout:
 * ┌─────────────────────────────────────────────────────────────┐
 * │                      Header Bar                              │
 * │  OZONE STUDIO | AGI Framework | [Consciousness: ON/OFF]     │
 * ├───────────────┬─────────────────────────────────────────────┤
 * │               │                                             │
 * │  Meta Portion │              Theme Area                     │
 * │  (20% width)  │         (Pipeline-driven UI)                │
 * │               │                                             │
 * │  - Home       │  - Workspaces                               │
 * │  - Prompt     │  - Pipeline outputs                         │
 * │  - Voice      │  - Task results                             │
 * │  - Tasks      │                                             │
 * │               │                                             │
 * ├───────────────┴─────────────────────────────────────────────┤
 * │                      Status Bar (BOTTOM)                     │
 * │  [●] Online | Peers: 42 | Contributions: 1,234 | ZSEI: 3   │
 * └─────────────────────────────────────────────────────────────┘
 */
function App() {
  const { 
    isConnected, 
    currentTheme,
    metaPortionWidth,
    consciousnessEnabled,
    initializeApp,
    setConnectionStatus,
    setSystemStats,
  } = useOzoneStore();
  
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function init() {
      try {
        setLoading(true);
        
        // Check if we're in Electron with backend available
        if (!window.ozone) {
          console.warn('Running without backend - limited functionality');
          setConnectionStatus(false);
          await initializeApp({});
          setLoading(false);
          return;
        }
        
        // Try to connect to backend
        try {
          const config = await window.ozone.config.get();
          await initializeApp(config);
          setConnectionStatus(true);
          
          // Fetch initial system stats
          if (window.ozone.system?.getStats) {
            const stats = await window.ozone.system.getStats();
            setSystemStats(stats);
          }
        } catch (configErr) {
          console.warn('Backend not available:', configErr);
          setConnectionStatus(false);
          await initializeApp({});
        }
        
        // Subscribe to events
        if (window.ozone?.events) {
          window.ozone.events.onBackendError((data: any) => {
            console.error('Backend error:', data);
          });
          
          window.ozone.events.onConnectionChange((data) => {
            setConnectionStatus(data.connected);
          });
          
          if (window.ozone.events.onStatsUpdate) {
            window.ozone.events.onStatsUpdate((stats) => {
              setSystemStats(stats);
            });
          }
        }
        
        setLoading(false);
      } catch (err) {
        console.error('Initialization failed:', err);
        setConnectionStatus(false);
        setLoading(false);
      }
    }
    
    init();
  }, [initializeApp, setConnectionStatus, setSystemStats]);

  // Brief loading splash
  if (loading) {
    return (
      <div className="app-loading">
        <div className="loading-logo">
          <span className="logo-icon">◎</span>
          <span className="logo-text">OZONE</span>
        </div>
        <div className="loading-spinner" />
        <p className="loading-subtitle">Omnidirectional Zero-Shot Neural Engine</p>
      </div>
    );
  }

  return (
    <div className="app">
      {/* Header Bar */}
      <header className="header-bar">
        <div className="header-brand">
          <span className="brand-icon">◎</span>
          <span className="brand-name">OZONE STUDIO</span>
          <span className="brand-version">v0.3</span>
        </div>
        
        <div className="header-tagline">
          Omnidirectional Zero-Shot Neural Engine
        </div>
        
        <div className="header-features">
          <div className={`feature-badge ${consciousnessEnabled ? 'active' : 'inactive'}`}>
            <span className="badge-icon">{consciousnessEnabled ? '🧠' : '○'}</span>
            <span className="badge-label">Consciousness</span>
            <span className="badge-status">{consciousnessEnabled ? 'ON' : 'OFF'}</span>
          </div>
          
          <div className={`feature-badge ${isConnected ? 'active' : 'inactive'}`}>
            <span className="badge-icon">{isConnected ? '🌐' : '○'}</span>
            <span className="badge-label">P2P Network</span>
            <span className="badge-status">{isConnected ? 'ON' : 'OFF'}</span>
          </div>
        </div>
      </header>
      
      {/* Main Content Area */}
      <div className="app-content">
        {/* Meta Portion - Always accessible */}
        <MetaPortion width={metaPortionWidth} />
        
        {/* Theme Area - Pipeline-driven content */}
        <ThemeArea theme={currentTheme} />
      </div>
      
      {/* Status Bar - BOTTOM (Shows Collective Contributions) */}
      <StatusBar />
    </div>
  );
}

export default App;
