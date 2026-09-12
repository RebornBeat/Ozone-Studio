/**
 * Ozone Studio - Main React Application v0.4.0
 *
 * OZONE STUDIO — Omnidirectional Zero-Shot Neural Engine
 * A Collective AGI Framework with Optional Consciousness
 */

import React, { useEffect, useState, useRef } from "react";
import { useOzoneStore } from "./services/store";
import { MetaPortion } from "./components/MetaPortion";
import { ThemeArea } from "./components/ThemeArea";
import { StatusBar } from "./components/StatusBar";
import ConnectedAgents from "./components/ConnectedAgents";
import { OZONE_HOST, fetchRemotePipelines } from "./ozoneClient";
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
  memoryUsage: number;
  activeTaskCount: number;
}

// Setup Wizard Types
interface SetupConfig {
  modelType: "api" | "local" | "zcode" | null;
  apiProvider: "anthropic" | "openai" | "google" | "openrouter" | "custom" | "local" | "";
  /** Only used when apiProvider === "custom". */
  apiEndpoint: string;
  apiKey: string;
  localModelPath: string;
  localModelType: "gguf" | "bitnet" | "other";
  /** Only used when localModelType === "bitnet" — pipeline 9 needs both the
   * llama.cpp-fork CLI binary AND the model file to actually run BitNet;
   * previously only the model file was ever asked for, so selecting BitNet
   * here was a dead end. */
  bitnetCliPath: string;
  voiceEnabled: boolean;
  /** Voice backend — mirrors config VoiceConfig.backend. */
  voiceBackend: "whisper_rs" | "whisper_cpp" | "api";
  whisperModelPath: string;
  whisperCppPath: string;
  voiceApiEndpoint: string;
  consciousnessEnabled: boolean;
  enableP2p: boolean;
  enableMdns: boolean;
}

/// Default local whisper model (ggml-base.en) — exists on this machine.
const DEFAULT_WHISPER_MODEL = "/home/rebornbeat/ozone-models/whisper/ggml-base.en.bin";

interface WhisperModelStatus {
  name: string;
  size: string;
  installed: boolean;
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
    setConsciousnessEnabled,
  } = useOzoneStore();

  const validateApiKeyFormat = (
    provider: string,
    apiKey: string,
  ): string | null => {
    const key = apiKey.trim();

    if (!key) {
      return "Please enter an API key";
    }

    switch (provider) {
      case "openai":
        // OpenAI keys usually start with "sk-"
        if (!/^sk-[A-Za-z0-9\-_]{20,}$/.test(key)) {
          return 'Invalid OpenAI API key format. Expected: starts with "sk-" followed by alphanumeric characters.';
        }
        break;

      case "anthropic":
        // Anthropic keys usually start with "sk-ant-"
        if (!/^sk-ant-[A-Za-z0-9\-_]{20,}$/.test(key)) {
          return 'Invalid Anthropic API key format. Expected: starts with "sk-ant-" followed by alphanumeric characters.';
        }
        break;

      case "google":
        // Google AI keys are usually long alphanumeric strings
        if (!/^[A-Za-z0-9_\-]{30,}$/.test(key)) {
          return "Invalid Google API key format. Expected: a long alphanumeric string.";
        }
        break;

      default:
        return "Unknown API provider selected";
    }

    return null; // valid
  };

  const validateModelPath = (
    modelType: string,
    filePath: string,
  ): string | null => {
    const path = filePath.trim();

    if (!path) {
      return "Please select a model file using the Browse button";
    }

    const lower = path.toLowerCase();

    if (modelType === "gguf" && !lower.endsWith(".gguf")) {
      return "Selected file must be a .gguf file for llama.cpp compatible models";
    }

    if (modelType === "bitnet" && !lower.endsWith(".gguf")) {
      return "BitNet models are expected to be in .gguf format";
    }

    if (modelType === "other" && !lower.match(/\.(gguf|bin|safetensors)$/)) {
      return "Supported file types: .gguf, .bin, .safetensors";
    }

    return null; // path looks acceptable
  };

  const handleSelectWhisperFile = async () => {
    if (window.ozone?.system?.selectFile) {
      const path = await window.ozone.system.selectFile({
        filters: [".bin", ".pt", ".pth"], // whisper models are usually pytorch .bin / .pt
      });
      if (path) {
        setSetupConfig((prev) => ({ ...prev, whisperModelPath: path }));
      }
    }
  };

  const [loading, setLoading] = useState(true);
  const [countdown, setCountdown] = useState<CountdownData | null>(null);
  const [launchStatus, setLaunchStatus] = useState<LaunchStatus | null>(null);
  const [showSetupWizard, setShowSetupWizard] = useState(false);
  const [setupStep, setSetupStep] = useState(0);
  const [setupErrors, setSetupErrors] = useState<string[]>([]);
  const [setupConfig, setSetupConfig] = useState<SetupConfig>({
    modelType: null,
    apiProvider: "",
    apiEndpoint: "",
    apiKey: "",
    localModelPath: "",
    localModelType: "gguf",
    bitnetCliPath: "",
    voiceEnabled: false,
    voiceBackend: "whisper_rs",
    whisperModelPath: DEFAULT_WHISPER_MODEL,
    whisperCppPath: "",
    voiceApiEndpoint: "",
    consciousnessEnabled: false,
    enableP2p: true,
    enableMdns: true,
  });
  const [whisperModelPath, setWhisperModelPath] = useState<string>("");
  // Monitor/Tools/Devices used to live behind a bottom-drawer toggle here;
  // they're now first-class core tabs (pipeline-ui.tsx CORE_TAB_DEFINITIONS)
  // so they inherit the same full-height layout as Workspace/Settings
  // instead of a cramped fixed-height strip.
  // Live probe: which model-role agents are registered with the host right
  // now (drives the wizard's honest ZCode status for ANY user).
  const [modelAgents, setModelAgents] = useState<string[]>([]);
  useEffect(() => {
    if (!showSetupWizard) return;
    let cancelled = false;
    const probe = async () => {
      try {
        const out = await fetchRemotePipelines();
        if (cancelled) return;
        setModelAgents(
          (out.pipelines ?? [])
            .filter((p) => (p.roles ?? ["agent"]).includes("model"))
            .map((p) => p.name),
        );
      } catch {
        if (!cancelled) setModelAgents([]);
      }
    };
    probe();
    const t = setInterval(probe, 5000);
    return () => {
      cancelled = true;
      clearInterval(t);
    };
  }, [showSetupWizard]);

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

  // Setup wizard validation
  const validateSetupStep = async (): Promise<boolean> => {
    const errors: string[] = [];

    if (setupStep === 0) {
      // Model configuration — ZCode connects via the host registry, nothing
      // to type; api needs provider + key; local needs a model file.
      if (!setupConfig.modelType) {
        errors.push("Please select a model type");
      } else if (setupConfig.modelType === "zcode") {
        // valid by construction — ZCode is already connected to this host
      } else if (setupConfig.modelType === "api") {
        if (!setupConfig.apiProvider) {
          errors.push("Please select an API provider");
        }
        const apiKeyError = validateApiKeyFormat(
          setupConfig.apiProvider,
          setupConfig.apiKey,
        );

        if (apiKeyError) {
          errors.push(apiKeyError);
        }
      } else if (setupConfig.modelType === "local") {
        const modelError = validateModelPath(
          setupConfig.localModelType,
          setupConfig.localModelPath,
        );

        if (modelError) {
          errors.push(modelError);
        }
      }
    }

    if (setupStep === 1) {
      if (setupConfig.voiceEnabled) {
        if (
          setupConfig.voiceBackend === "whisper_rs" ||
          setupConfig.voiceBackend === "whisper_cpp"
        ) {
          if (!setupConfig.whisperModelPath?.trim()) {
            errors.push(
              "Please select a Whisper model file when voice input is enabled",
            );
          } else if (!/\.(bin|pt|pth|onnx|ggml)$/i.test(setupConfig.whisperModelPath)) {
            errors.push(
              "Selected file should have extension .bin, .pt, .pth, .onnx or .ggml",
            );
          }
          if (
            setupConfig.voiceBackend === "whisper_cpp" &&
            !setupConfig.whisperCppPath?.trim()
          ) {
            errors.push(
              "whisper_cpp backend needs the whisper-cli binary path",
            );
          }
        } else if (setupConfig.voiceBackend === "api") {
          if (!setupConfig.voiceApiEndpoint?.trim()) {
            errors.push("API voice backend needs a transcription endpoint");
          }
        }
      }
    }

    setSetupErrors(errors);
    return errors.length === 0;
  };

  // Handle file selection for local model
  const handleSelectModelFile = async () => {
    if (window.ozone?.system?.selectFile) {
      const path = await window.ozone.system.selectFile({
        filters: [".gguf", ".bin", ".safetensors"],
      });
      if (path) {
        setSetupConfig((prev) => ({ ...prev, localModelPath: path }));
      }
    }
  };

  // Navigate setup wizard
  const handleSetupNext = async () => {
    const isValid = await validateSetupStep(); // now async
    if (!isValid) return; // ← most important line

    const nextStep = setupStep + 1;
    if (nextStep < 4) {
      setSetupStep(nextStep);
    }
  };

  const handleSetupBack = () => {
    if (setupStep > 0) {
      setSetupStep(setupStep - 1);
      setSetupErrors([]);
    }
  };

  // Complete setup wizard
  const completeSetup = async () => {
    try {
      const configUpdates = {
        setup_complete: true,
        models: {
          // "local" here maps to whichever concrete backend the user picked
          // (bitnet/gguf/other) — pipeline 9 keys off that concrete type,
          // not the generic "local" the wizard uses as a UI grouping.
          model_type:
            setupConfig.modelType === "local"
              ? setupConfig.localModelType === "other"
                ? "gguf"
                : setupConfig.localModelType
              : setupConfig.modelType,
          api_provider:
            setupConfig.modelType === "api"
              ? setupConfig.apiProvider
              : undefined,
          api_endpoint:
            setupConfig.modelType === "api" &&
            setupConfig.apiProvider === "custom"
              ? setupConfig.apiEndpoint
              : undefined,
          api_key:
            setupConfig.modelType === "api" ? setupConfig.apiKey : undefined,
          wire_protocol:
            setupConfig.modelType === "api"
              ? setupConfig.apiProvider === "anthropic"
                ? "anthropic"
                : "chat_completions"
              : undefined,
          local_model_path:
            setupConfig.modelType === "local"
              ? setupConfig.localModelPath
              : undefined,
          local_model_type: setupConfig.localModelType,
          bitnet_cli_path:
            setupConfig.modelType === "local" &&
            setupConfig.localModelType === "bitnet"
              ? setupConfig.bitnetCliPath
              : undefined,
        },
        network: {
          enable_p2p: setupConfig.enableP2p,
          enable_mdns: setupConfig.enableP2p ? setupConfig.enableMdns : false,
        },
        voice: {
          enabled: setupConfig.voiceEnabled,
          backend: setupConfig.voiceBackend,
          whisper_model_path:
            setupConfig.voiceEnabled &&
            setupConfig.voiceBackend !== "api"
              ? setupConfig.whisperModelPath
              : undefined,
          whisper_cpp_path:
            setupConfig.voiceEnabled &&
            setupConfig.voiceBackend === "whisper_cpp"
              ? setupConfig.whisperCppPath
              : undefined,
          api_endpoint:
            setupConfig.voiceEnabled && setupConfig.voiceBackend === "api"
              ? setupConfig.voiceApiEndpoint
              : undefined,
        },
        consciousness: {
          enabled: setupConfig.consciousnessEnabled,
        },
      };

      if (window.ozone?.config?.set) {
        const result = await window.ozone.config.set(configUpdates);
        if (!result.success) {
          throw new Error(result.error || "Failed to save configuration");
        }
      }

      if (window.ozone?.system?.markSetupComplete) {
        await window.ozone.system.markSetupComplete();
      }

      // Update local state
      setConsciousnessEnabled(setupConfig.consciousnessEnabled);
      setShowSetupWizard(false);
    } catch (err) {
      console.error("Setup completion failed:", err);
      setSetupErrors(["Failed to save configuration. Please try again."]);
    }
  };

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
    return (
      <div className="setup-wizard">
        <div className="setup-container">
          <div className="setup-header">
            <span className="logo-icon">◎</span>
            <h1>Welcome to OZONE STUDIO</h1>
            <p>Let's configure your environment</p>
          </div>

          <div className="setup-progress">
            <div
              className={`progress-step ${setupStep >= 0 ? "active" : ""} ${setupStep > 0 ? "complete" : ""}`}
            >
              <span className="step-number">1</span>
              <span className="step-label">Model</span>
            </div>
            <div
              className={`progress-step ${setupStep >= 1 ? "active" : ""} ${setupStep > 1 ? "complete" : ""}`}
            >
              <span className="step-number">2</span>
              <span className="step-label">Voice</span>
            </div>
            <div
              className={`progress-step ${setupStep >= 2 ? "active" : ""} ${setupStep > 2 ? "complete" : ""}`}
            >
              <span className="step-number">3</span>
              <span className="step-label">Features</span>
            </div>
            <div className={`progress-step ${setupStep >= 3 ? "active" : ""}`}>
              <span className="step-number">4</span>
              <span className="step-label">Done</span>
            </div>
          </div>

          {setupErrors.length > 0 && (
            <div className="setup-errors">
              {setupErrors.map((error, i) => (
                <p key={i} className="error-message">
                  ⚠️ {error}
                </p>
              ))}
            </div>
          )}

          <div className="setup-content">
            {setupStep === 0 && (
              <div className="setup-step">
                <h2>🤖 Model Configuration</h2>
                <p>Choose how OZONE will process your prompts:</p>

                <div className="model-type-selection">
                  <button
                    className={`model-type-btn ${setupConfig.modelType === "zcode" ? "selected" : ""}`}
                    onClick={() =>
                      setSetupConfig((prev) => ({ ...prev, modelType: "zcode" }))
                    }
                  >
                    <span className="btn-icon">⚡</span>
                    <span className="btn-title">ZCode (Connected)</span>
                    <span className="btn-desc">
                      ZCode serves as a model via the Ozone-Studio registry
                    </span>
                  </button>

                  <button
                    className={`model-type-btn ${setupConfig.modelType === "api" ? "selected" : ""}`}
                    onClick={() =>
                      setSetupConfig((prev) => ({ ...prev, modelType: "api" }))
                    }
                  >
                    <span className="btn-icon">🌐</span>
                    <span className="btn-title">API Model</span>
                    <span className="btn-desc">
                      Claude, GPT, or other API services
                    </span>
                  </button>

                  <button
                    className={`model-type-btn ${setupConfig.modelType === "local" ? "selected" : ""}`}
                    onClick={() =>
                      setSetupConfig((prev) => ({
                        ...prev,
                        modelType: "local",
                      }))
                    }
                  >
                    <span className="btn-icon">💻</span>
                    <span className="btn-title">Local Model</span>
                    <span className="btn-desc">
                      Run models on your own hardware
                    </span>
                  </button>
                </div>

                {setupConfig.modelType === "zcode" && (
                  <div className="model-config-section">
                    {modelAgents.includes("zcode") ? (
                      <p className="config-hint">
                        ✓ <b>ZCode is connected</b> to this host (roles: agent +
                        model) — model calls dispatch over the registry. Nothing
                        to type here.
                      </p>
                    ) : modelAgents.length > 0 ? (
                      <p className="config-hint">
                        ✓ A model agent is connected:{" "}
                        <b>{modelAgents.join(", ")}</b> — model calls dispatch
                        over the registry.
                      </p>
                    ) : (
                      <p className="config-hint">
                        ZCode runs as a model through the <b>ZCode connector</b>{" "}
                        — a tiny zero-dependency agent that registers this
                        machine with the host and stays connected
                        (auto-reconnect). Start it with:
                        <br />
                        <code>node tools/zcode-connector/connect.js watch</code>
                        <br />
                        (Node 18+). This panel turns green the moment it lands —
                        the host and the UI don't need restarting.
                      </p>
                    )}
                  </div>
                )}

                {setupConfig.modelType === "api" && (
                  <div className="model-config-section">
                    <label>API Provider</label>
                    <div className="api-provider-selection">
                      <button
                        className={`provider-btn ${setupConfig.apiProvider === "anthropic" ? "selected" : ""}`}
                        onClick={() =>
                          setSetupConfig((prev) => ({
                            ...prev,
                            apiProvider: "anthropic",
                          }))
                        }
                      >
                        <span className="provider-name">Anthropic</span>
                        <span className="provider-model">Claude</span>
                      </button>
                      <button
                        className={`provider-btn ${setupConfig.apiProvider === "openai" ? "selected" : ""}`}
                        onClick={() =>
                          setSetupConfig((prev) => ({
                            ...prev,
                            apiProvider: "openai",
                          }))
                        }
                      >
                        <span className="provider-name">OpenAI</span>
                        <span className="provider-model">GPT-4</span>
                      </button>
                      <button
                        className={`provider-btn ${setupConfig.apiProvider === "google" ? "selected" : ""}`}
                        onClick={() =>
                          setSetupConfig((prev) => ({
                            ...prev,
                            apiProvider: "google",
                          }))
                        }
                      >
                        <span className="provider-name">Google</span>
                        <span className="provider-model">Gemini</span>
                      </button>
                      <button
                        className={`provider-btn ${setupConfig.apiProvider === "openrouter" ? "selected" : ""}`}
                        onClick={() =>
                          setSetupConfig((prev) => ({
                            ...prev,
                            apiProvider: "openrouter",
                          }))
                        }
                      >
                        <span className="provider-name">OpenRouter</span>
                        <span className="provider-model">Many models</span>
                      </button>
                      <button
                        className={`provider-btn ${setupConfig.apiProvider === "custom" ? "selected" : ""}`}
                        onClick={() =>
                          setSetupConfig((prev) => ({
                            ...prev,
                            apiProvider: "custom",
                          }))
                        }
                      >
                        <span className="provider-name">Custom</span>
                        <span className="provider-model">Any OpenAI-compatible endpoint</span>
                      </button>
                    </div>

                    {setupConfig.apiProvider === "custom" && (
                      <>
                        <label>API Endpoint</label>
                        <input
                          type="text"
                          placeholder="https://…/v1/chat/completions"
                          value={setupConfig.apiEndpoint}
                          onChange={(e) =>
                            setSetupConfig((prev) => ({
                              ...prev,
                              apiEndpoint: e.target.value,
                            }))
                          }
                        />
                      </>
                    )}

                    <label>API Key</label>
                    <input
                      type="password"
                      placeholder={
                        setupConfig.apiProvider
                          ? `Enter your ${setupConfig.apiProvider} API key`
                          : "Select a provider first"
                      }
                      value={setupConfig.apiKey}
                      onChange={(e) =>
                        setSetupConfig((prev) => ({
                          ...prev,
                          apiKey: e.target.value,
                        }))
                      }
                      disabled={!setupConfig.apiProvider}
                    />
                    {setupConfig.apiProvider === "anthropic" && (
                      <p className="config-hint">
                        Get your API key from console.anthropic.com
                      </p>
                    )}
                    {setupConfig.apiProvider === "openai" && (
                      <p className="config-hint">
                        Get your API key from platform.openai.com
                      </p>
                    )}
                    {setupConfig.apiProvider === "google" && (
                      <p className="config-hint">
                        Get your API key from aistudio.google.com
                      </p>
                    )}
                    {setupConfig.apiProvider === "openrouter" && (
                      <p className="config-hint">
                        Get your API key from openrouter.ai/keys — pick any
                        model identifier in Settings afterward (e.g.
                        anthropic/claude-3.5-sonnet, deepseek/deepseek-coder).
                      </p>
                    )}
                  </div>
                )}

                {setupConfig.modelType === "local" && (
                  <div className="model-config-section">
                    <label>Model Type</label>
                    <div className="local-model-types">
                      <button
                        className={`local-type-btn ${setupConfig.localModelType === "gguf" ? "selected" : ""}`}
                        onClick={() =>
                          setSetupConfig((prev) => ({
                            ...prev,
                            localModelType: "gguf",
                          }))
                        }
                      >
                        GGUF (llama.cpp)
                      </button>
                      <button
                        className={`local-type-btn ${setupConfig.localModelType === "bitnet" ? "selected" : ""}`}
                        onClick={() =>
                          setSetupConfig((prev) => ({
                            ...prev,
                            localModelType: "bitnet",
                          }))
                        }
                      >
                        BitNet (1-bit)
                      </button>
                      <button
                        className={`local-type-btn ${setupConfig.localModelType === "other" ? "selected" : ""}`}
                        onClick={() =>
                          setSetupConfig((prev) => ({
                            ...prev,
                            localModelType: "other",
                          }))
                        }
                      >
                        Other
                      </button>
                    </div>

                    <label>Model File</label>
                    <div className="file-input-group">
                      <input
                        type="text"
                        placeholder="Select model file using Browse →"
                        value={setupConfig.localModelPath}
                        readOnly // ← prevents typing
                        className="readonly-path-input" // optional: add styling
                        onClick={handleSelectModelFile} // optional: clicking input also opens dialog
                      />
                      <button
                        className="browse-btn"
                        onClick={handleSelectModelFile}
                      >
                        Browse...
                      </button>
                    </div>
                    {setupConfig.localModelType === "bitnet" && (
                      <>
                        <label>BitNet CLI (llama-cli from the BitNet build)</label>
                        <input
                          type="text"
                          placeholder="/path/to/BitNet/build/bin/llama-cli"
                          value={setupConfig.bitnetCliPath}
                          onChange={(e) =>
                            setSetupConfig((prev) => ({
                              ...prev,
                              bitnetCliPath: e.target.value,
                            }))
                          }
                        />
                        <p className="config-hint">
                          BitNet needs both this CLI binary and the model file
                          above — pipeline 9 shells out to it directly.
                        </p>
                      </>
                    )}
                    <p className="config-hint">
                      {setupConfig.localModelType === "gguf" &&
                        "Recommended: Llama 3, Mistral, or Phi-3 in GGUF format"}
                      {setupConfig.localModelType === "bitnet" &&
                        "BitNet models offer fast 1-bit inference"}
                      {setupConfig.localModelType === "other" &&
                        "Ensure the model format is compatible with your runtime"}
                    </p>
                  </div>
                )}

                <div className="setup-buttons">
                  <button
                    className="setup-next"
                    onClick={handleSetupNext}
                    disabled={!setupConfig.modelType}
                  >
                    Continue →
                  </button>
                </div>
              </div>
            )}

            {setupStep === 1 && (
              <div className="setup-step">
                <h2>🎤 Voice Configuration</h2>
                <p>Enable voice input for hands-free interaction (optional)</p>

                <div className="voice-toggle">
                  <label className="toggle-label">
                    <input
                      type="checkbox"
                      checked={setupConfig.voiceEnabled}
                      onChange={(e) =>
                        setSetupConfig((prev) => ({
                          ...prev,
                          voiceEnabled: e.target.checked,
                        }))
                      }
                    />
                    <span className="toggle-text">Enable Voice Input</span>
                  </label>
                </div>

                {setupConfig.voiceEnabled && (
                  <div className="voice-config-section">
                    <label>Voice Backend</label>
                    <div className="local-model-types">
                      <button
                        className={`local-type-btn ${setupConfig.voiceBackend === "whisper_rs" ? "selected" : ""}`}
                        onClick={() =>
                          setSetupConfig((prev) => ({
                            ...prev,
                            voiceBackend: "whisper_rs",
                          }))
                        }
                      >
                        whisper-rs (integrated)
                      </button>
                      <button
                        className={`local-type-btn ${setupConfig.voiceBackend === "whisper_cpp" ? "selected" : ""}`}
                        onClick={() =>
                          setSetupConfig((prev) => ({
                            ...prev,
                            voiceBackend: "whisper_cpp",
                          }))
                        }
                      >
                        whisper.cpp (CLI)
                      </button>
                      <button
                        className={`local-type-btn ${setupConfig.voiceBackend === "api" ? "selected" : ""}`}
                        onClick={() =>
                          setSetupConfig((prev) => ({
                            ...prev,
                            voiceBackend: "api",
                          }))
                        }
                      >
                        API
                      </button>
                    </div>

                    {setupConfig.voiceBackend === "api" ? (
                      <div className="model-config-section">
                        <label>Transcription Endpoint</label>
                        <input
                          type="text"
                          placeholder="https://…/v1/audio/transcriptions"
                          value={setupConfig.voiceApiEndpoint}
                          onChange={(e) =>
                            setSetupConfig((prev) => ({
                              ...prev,
                              voiceApiEndpoint: e.target.value,
                            }))
                          }
                        />
                      </div>
                    ) : (
                      <>
                        <label>Whisper Model File</label>
                        <div className="file-input-group">
                          <input
                            type="text"
                            placeholder="No file selected"
                            value={setupConfig.whisperModelPath}
                            readOnly={
                              setupConfig.whisperModelPath === DEFAULT_WHISPER_MODEL
                            }
                            onClick={handleSelectWhisperFile}
                          />
                          <button
                            className="browse-btn"
                            onClick={handleSelectWhisperFile}
                          >
                            Browse…
                          </button>
                        </div>
                        <p className="config-hint">
                          Default: ggml-base.en detected on this machine
                          {" "}`{DEFAULT_WHISPER_MODEL}`
                        </p>
                        {setupConfig.voiceBackend === "whisper_cpp" && (
                          <div className="model-config-section">
                            <label>whisper-cli Binary Path</label>
                            <input
                              type="text"
                              placeholder="/usr/local/bin/whisper-cli"
                              value={setupConfig.whisperCppPath}
                              onChange={(e) =>
                                setSetupConfig((prev) => ({
                                  ...prev,
                                  whisperCppPath: e.target.value,
                                }))
                              }
                            />
                          </div>
                        )}
                      </>
                    )}
                  </div>
                )}

                <div className="setup-buttons">
                  <button className="setup-back" onClick={handleSetupBack}>
                    ← Back
                  </button>
                  <button className="setup-next" onClick={handleSetupNext}>
                    Continue →
                  </button>
                </div>
              </div>
            )}

            {setupStep === 2 && (
              <div className="setup-step">
                <h2>🧠 Consciousness Features</h2>
                <p>
                  Enable AGI-like consciousness features (optional, can enable
                  later)
                </p>

                <div className="consciousness-toggle">
                  <label className="toggle-label">
                    <input
                      type="checkbox"
                      checked={setupConfig.consciousnessEnabled}
                      onChange={(e) =>
                        setSetupConfig((prev) => ({
                          ...prev,
                          consciousnessEnabled: e.target.checked,
                        }))
                      }
                    />
                    <span className="toggle-text">
                      Enable Consciousness System
                    </span>
                  </label>
                </div>

                <div className="consciousness-features">
                  <h4>When enabled, includes:</h4>
                  <ul>
                    <li>
                      🎭 Emotional Context - Responses with emotional awareness
                    </li>
                    <li>📚 Experience Memory - Learning from interactions</li>
                    <li>🔄 I-Loop Reflection - Self-improvement cycle</li>
                    <li>
                      🤝 Relationship Development - Personalized interactions
                    </li>
                    <li>⚖️ Ethical Framework - Value-aligned responses</li>
                  </ul>
                </div>

                <p className="feature-note">
                  💡 You can always enable or disable this later in Settings
                </p>

                <div className="consciousness-toggle" style={{ marginTop: 18 }}>
                  <label className="toggle-label">
                    <input
                      type="checkbox"
                      checked={setupConfig.enableP2p}
                      onChange={(e) =>
                        setSetupConfig((prev) => ({
                          ...prev,
                          enableP2p: e.target.checked,
                        }))
                      }
                    />
                    <span className="toggle-text">
                      Enable P2P networking (collective knowledge sharing)
                    </span>
                  </label>
                </div>
                {setupConfig.enableP2p && (
                  <div className="consciousness-toggle">
                    <label className="toggle-label">
                      <input
                        type="checkbox"
                        checked={setupConfig.enableMdns}
                        onChange={(e) =>
                          setSetupConfig((prev) => ({
                            ...prev,
                            enableMdns: e.target.checked,
                          }))
                        }
                      />
                      <span className="toggle-text">
                        Enable local discovery (mDNS)
                      </span>
                    </label>
                  </div>
                )}
                <p className="feature-note">
                  💡 Turn this off for fully local, single-machine operation —
                  configurable later in Settings → Network
                </p>

                <div className="setup-buttons">
                  <button className="setup-back" onClick={handleSetupBack}>
                    ← Back
                  </button>
                  <button className="setup-next" onClick={handleSetupNext}>
                    Continue →
                  </button>
                </div>
              </div>
            )}

            {setupStep === 3 && (
              <div className="setup-step">
                <h2>✅ Setup Complete!</h2>
                <p>Your configuration summary:</p>

                <div className="setup-summary">
                  <div className="summary-item">
                    <span className="summary-label">Model:</span>
                    <span className="summary-value">
                      {setupConfig.modelType === "zcode"
                        ? "ZCode (registry model)"
                        : setupConfig.modelType === "api"
                          ? "API Model"
                          : `Local ${setupConfig.localModelType.toUpperCase()}`}
                    </span>
                  </div>
                  <div className="summary-item">
                    <span className="summary-label">Voice Input:</span>
                    <span className="summary-value">
                      {setupConfig.voiceEnabled
                        ? `Enabled (${
                            setupConfig.voiceBackend === "api"
                              ? "API backend"
                              : setupConfig.voiceBackend === "whisper_rs"
                                ? "whisper-rs integrated"
                                : "whisper.cpp CLI"
                          })`
                        : "Disabled"}
                    </span>
                  </div>
                  <div className="summary-item">
                    <span className="summary-label">Consciousness:</span>
                    <span className="summary-value">
                      {setupConfig.consciousnessEnabled
                        ? "Enabled"
                        : "Disabled"}
                    </span>
                  </div>
                </div>

                <p className="summary-note">
                  All settings can be changed anytime in the Settings tab.
                </p>

                <div className="setup-buttons">
                  <button className="setup-back" onClick={handleSetupBack}>
                    ← Back
                  </button>
                  <button className="setup-complete" onClick={completeSetup}>
                    Start Using OZONE STUDIO →
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    );
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
