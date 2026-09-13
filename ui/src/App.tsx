/**
 * Ozone Studio - Main React Application v0.4.0
 *
 * OZONE STUDIO — Omnidirectional Zero-Shot Neural Engine
 * A Collective AGI Framework with Optional Consciousness
 */

import React, { useEffect, useState } from "react";
import { useOzoneStore } from "./services/store";
import { MetaPortion } from "./components/MetaPortion";
import { ThemeArea } from "./components/ThemeArea";
import { StatusBar } from "./components/StatusBar";
import ConnectedAgents from "./components/ConnectedAgents";
import { SetupWizard } from "./components/SetupWizard";
import { OZONE_HOST } from "./ozoneClient";
import "./App.css";

// TypeScript declarations
declare global {
  interface Window {
    ozone?: {
      auth: {
        challenge: (
          publicKey: Uint8Array,
        ) => Promise<{ challenge: Uint8Array }>;
        authenticate: (
          publicKey: Uint8Array,
          signature: Uint8Array,
        ) => Promise<{
          success: boolean;
          sessionToken?: Uint8Array;
          error?: string;
        }>;
      };
      pipeline: {
        execute: (
          pipelineId: number,
          input: object,
        ) => Promise<{
          success: boolean;
          taskId: number;
          output?: object;
          error?: string;
        }>;
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
        get: () => Promise<any>;
        set: (updates: object) => Promise<{ success: boolean }>;
      };
      system: {
        getStats: () => Promise<SystemStats>;
        isFirstLaunch: () => Promise<boolean>;
        markSetupComplete: () => Promise<{ success: boolean }>;
        selectFile: (options?: {
          filters?: string[];
        }) => Promise<string | null>;
      };
      events: {
        onBackendError: (callback: (data: object) => void) => void;
        onConnectionChange: (
          callback: (data: { connected: boolean }) => void,
        ) => void;
        onConnectionCountdown: (
          callback: (data: CountdownData) => void,
        ) => void;
        onBackendLaunchStatus: (callback: (data: LaunchStatus) => void) => void;
        onStatsUpdate: (callback: (data: SystemStats) => void) => void;
      };
    };
  }
}

interface CountdownData {
  secondsUntilRetry: number;
  willAutoLaunch: boolean;
  hasAttemptedLaunch: boolean;
}

interface LaunchStatus {
  success: boolean;
  message?: string;
  error?: string;
  path?: string;
}

export interface SystemStats {
  backendConnected: boolean;
  // p2pEnabled/peerCount are now real — sourced from GET /health's
  // p2p_enabled/peer_count fields (NetworkManager::get_status). The
  // contribution/ZSEI counters below are still placeholders: no backend
  // field or endpoint tracks them yet (a real collective-stats system is
  // separate, larger work) — kept in the UI as the intended future surface,
  // not deleted, and always render as 0 until that lands.
  p2pEnabled: boolean;
  peerCount: number;
  totalContributions: number;
  myContributions: number;
  methodologiesShared: number;
  blueprintsShared: number;
  findingsShared: number;
  zseiContainers: number;
  zseiDepth: number;
  consciousnessEnabled: boolean;
  consciousnessState?: string;
  iLoopStatus?: string;
  uptime: number;
  /** This Electron UI process's own heap % — not the backend. */
  memoryUsage: number;
  /** Real Rust backend process RSS (MB) / CPU% — found by whoever's
   * listening on the backend port, so this works whether the backend was
   * launched by this app or manually in a separate terminal. Null while
   * unavailable (e.g. non-Linux, or the backend just isn't up yet). */
  backendMemoryMb: number | null;
  backendCpuPercent: number | null;
  activeTaskCount: number;
}

function App() {
  const {
    isConnected,
    currentTheme,
    consciousnessEnabled,
    p2pEnabled,
    initializeApp,
    setConnectionStatus,
    setSystemStats,
  } = useOzoneStore();

  const [loading, setLoading] = useState(true);
  const [countdown, setCountdown] = useState<CountdownData | null>(null);
  const [launchStatus, setLaunchStatus] = useState<LaunchStatus | null>(null);
  // Setup wizard is fully self-contained (components/SetupWizard.tsx) — this
  // is the only bit of its state that belongs here, since it decides
  // whether App renders the wizard or the main UI.
  const [showSetupWizard, setShowSetupWizard] = useState(false);

  // Try to connect to backend
  const tryConnect = async (): Promise<boolean> => {
    if (!window.ozone) return false;

    try {
      const config = await window.ozone.config.get();
      await initializeApp(config);
      setConnectionStatus(true);

      if (window.ozone.system?.getStats) {
        const stats = await window.ozone.system.getStats();
        setSystemStats(stats);
      }

      // Check if first launch
      if (window.ozone.system?.isFirstLaunch) {
        const isFirst = await window.ozone.system.isFirstLaunch();
        setShowSetupWizard(isFirst);
      }

      return true;
    } catch (err) {
      return false;
    }
  };

  useEffect(() => {
    async function init() {
      setLoading(true);

      if (!window.ozone) {
        console.warn("Running in browser mode - no backend API");
        // WEB MODE: connect to the host over HTTP directly — the same
        // backend the Electron bridge wraps. /health is the probe; while
        // it's down the connecting screen is a REAL wait, retried until
        // the host appears (no more infinite loop with a live backend).
        setLoading(false);
        const probe = async (): Promise<boolean> => {
          try {
            const res = await fetch(`${OZONE_HOST}/health`);
            if (res.ok) {
              setConnectionStatus(true);
              // Web mode still initializes store state (consciousness flag,
              // model selection) from the live host config over HTTP.
              try {
                const cfgRes = await fetch(`${OZONE_HOST}/config/get`, {
                  method: "POST",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify({ section: "", session_token: "" }),
                });
                const cfg = await cfgRes.json();
                await initializeApp(cfg?.config ?? cfg);
              } catch {
                /* config optional — connection still stands */
              }
              return true;
            }
          } catch {
            /* host not reachable yet */
          }
          return false;
        };
        if (!(await probe())) {
          const retry = setInterval(async () => {
            if (await probe()) clearInterval(retry);
          }, 3000);
        }
        return;
      }

      // Subscribe to events
      if (window.ozone.events) {
        window.ozone.events.onConnectionChange((data) => {
          setConnectionStatus(data.connected);
          if (data.connected) {
            tryConnect();
          }
        });

        window.ozone.events.onConnectionCountdown?.((data) => {
          setCountdown(data);
        });

        window.ozone.events.onBackendLaunchStatus?.((data) => {
          setLaunchStatus(data);
        });

        window.ozone.events.onBackendError?.((data: any) => {
          console.error("Backend error:", data);
        });

        window.ozone.events.onStatsUpdate?.((stats) => {
          setSystemStats(stats);
        });
      }

      const connected = await tryConnect();
      setLoading(false);
    }

    init();
  }, []);

  // Loading splash
  if (loading) {
    return (
      <div className="app-loading">
        <div className="loading-logo">
          <span className="logo-icon">◎</span>
          <span className="logo-text">OZONE</span>
        </div>
        <div className="loading-spinner" />
        <p className="loading-subtitle">
          Omnidirectional Zero-Shot Neural Engine
        </p>
      </div>
    );
  }

  // Connecting screen with countdown
  if (!isConnected) {
    return (
      <div className="app-connecting">
        <div className="connecting-logo">
          <span className="logo-icon pulse">◎</span>
          <span className="logo-text">OZONE STUDIO</span>
        </div>
        <div className="loading-spinner" />
        <p className="connecting-status">
          Awaiting connection to OZONE STUDIO Backend...
        </p>

        {countdown &&
          !countdown.hasAttemptedLaunch &&
          countdown.secondsUntilRetry > 0 && (
            <p className="connecting-countdown">
              Auto-launching backend in{" "}
              <span className="countdown-number">
                {countdown.secondsUntilRetry}
              </span>{" "}
              seconds...
            </p>
          )}

        {countdown && countdown.hasAttemptedLaunch && (
          <p className="connecting-hint">
            {launchStatus?.success
              ? "Backend launched. Waiting for initialization..."
              : launchStatus?.error || "Waiting for manual backend start..."}
          </p>
        )}

        <div className="connecting-instructions">
          <p>Start the backend manually:</p>
          <code>cd target/release && ./ozone-studio</code>
        </div>
      </div>
    );
  }

  // Setup Wizard
  if (showSetupWizard) {
    return <SetupWizard onComplete={() => setShowSetupWizard(false)} />;
  }


  // Main UI
  return (
    <div className="app">
      <header className="header-bar">
        <div className="header-brand">
          <span className="brand-icon">◎</span>
          <span className="brand-name">OZONE STUDIO</span>
          <span className="brand-version">v0.4</span>
        </div>

        <div className="header-tagline">
          Omnidirectional Zero-Shot Neural Engine
        </div>

        <div className="header-features">
          <div
            className={`feature-badge ${consciousnessEnabled ? "active" : "inactive"}`}
          >
            <span className="badge-icon">🧠</span>
            <span className="badge-label">Consciousness</span>
            <span className="badge-status">
              {consciousnessEnabled ? "ON" : "OFF"}
            </span>
          </div>

          <div className={`feature-badge ${p2pEnabled ? "active" : "inactive"}`}>
            <span className="badge-icon">🌐</span>
            <span className="badge-label">P2P Network</span>
            <span className="badge-status">{p2pEnabled ? "ON" : "OFF"}</span>
          </div>

          <div className="feature-badge active" style={{ cursor: "default" }}>
            <ConnectedAgents compact pollMs={5000} />
          </div>
        </div>
      </header>

      <div className="app-content">
        <MetaPortion width={27} />
        <ThemeArea theme={currentTheme} />
      </div>

      <StatusBar />
    </div>
  );
}

export default App;
