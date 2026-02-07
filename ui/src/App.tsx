/**
 * Ozone Studio - Main React Application v0.4.0
 * 
 * OZONE STUDIO — Omnidirectional Zero-Shot Neural Engine
 * A Collective AGI Framework with Optional Consciousness
 */

import React, { useEffect, useState, useRef } from 'react';
import { useOzoneStore } from './services/store';
import { MetaPortion } from './components/MetaPortion';
import { ThemeArea } from './components/ThemeArea';
import { StatusBar } from './components/StatusBar';
import './App.css';

// TypeScript declarations
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
        get: () => Promise<any>;
        set: (updates: object) => Promise<{ success: boolean }>;
      };
      system: {
        getStats: () => Promise<SystemStats>;
        isFirstLaunch: () => Promise<boolean>;
        markSetupComplete: () => Promise<{ success: boolean }>;
        selectFile: (options?: { filters?: string[] }) => Promise<string | null>;
      };
      events: {
        onBackendError: (callback: (data: object) => void) => void;
        onConnectionChange: (callback: (data: { connected: boolean }) => void) => void;
        onConnectionCountdown: (callback: (data: CountdownData) => void) => void;
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
  modelType: 'api' | 'local' | null;
  apiProvider: 'anthropic' | 'openai' | 'google' | 'local' | '';
  apiKey: string;
  localModelPath: string;
  localModelType: 'gguf' | 'bitnet' | 'other';
  voiceEnabled: boolean;
  whisperModel: string;
  consciousnessEnabled: boolean;
}

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
    initializeApp,
    setConnectionStatus,
    setSystemStats,
    setConsciousnessEnabled,
  } = useOzoneStore();
  
  const [loading, setLoading] = useState(true);
  const [countdown, setCountdown] = useState<CountdownData | null>(null);
  const [launchStatus, setLaunchStatus] = useState<LaunchStatus | null>(null);
  const [showSetupWizard, setShowSetupWizard] = useState(false);
  const [setupStep, setSetupStep] = useState(0);
  const [setupErrors, setSetupErrors] = useState<string[]>([]);
  const [setupConfig, setSetupConfig] = useState<SetupConfig>({
    modelType: null,
    apiProvider: '',
    apiKey: '',
    localModelPath: '',
    localModelType: 'gguf',
    voiceEnabled: false,
    whisperModel: 'base',
    consciousnessEnabled: false,
  });
  const [whisperModels, setWhisperModels] = useState<WhisperModelStatus[]>([
    { name: 'tiny', size: '~75MB', installed: false },
    { name: 'base', size: '~150MB', installed: false },
    { name: 'small', size: '~500MB', installed: false },
    { name: 'medium', size: '~1.5GB', installed: false },
    { name: 'large', size: '~3GB', installed: false },
  ]);

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
        console.warn('Running in browser mode - no backend API');
        setLoading(false);
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
          console.error('Backend error:', data);
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
  const validateSetupStep = (): boolean => {
    const errors: string[] = [];
    
    if (setupStep === 0) {
      // Model configuration
      if (!setupConfig.modelType) {
        errors.push('Please select a model type');
      } else if (setupConfig.modelType === 'api') {
        if (!setupConfig.apiProvider) {
          errors.push('Please select an API provider');
        }
        if (!setupConfig.apiKey.trim()) {
          errors.push('Please enter an API key');
        }
      } else if (setupConfig.modelType === 'local' && !setupConfig.localModelPath.trim()) {
        errors.push('Please select or enter a model file path');
      }
    }
    
    if (setupStep === 1 && setupConfig.voiceEnabled) {
      // Check if selected whisper model is installed
      const selectedModel = whisperModels.find(m => m.name === setupConfig.whisperModel);
      if (selectedModel && !selectedModel.installed) {
        errors.push(`Whisper model "${setupConfig.whisperModel}" is not installed`);
      }
    }
    
    setSetupErrors(errors);
    return errors.length === 0;
  };

  // Check which whisper models are installed
  const checkWhisperModels = async () => {
    if (window.ozone?.system?.checkWhisperModels) {
      try {
        const installed = await window.ozone.system.checkWhisperModels();
        setWhisperModels(prev => prev.map(m => ({
          ...m,
          installed: installed.includes(m.name)
        })));
      } catch (e) {
        console.warn('Could not check whisper models');
      }
    }
  };

  // Handle file selection for local model
  const handleSelectModelFile = async () => {
    if (window.ozone?.system?.selectFile) {
      const path = await window.ozone.system.selectFile({
        filters: ['.gguf', '.bin', '.safetensors']
      });
      if (path) {
        setSetupConfig(prev => ({ ...prev, localModelPath: path }));
      }
    }
  };

  // Navigate setup wizard
  const handleSetupNext = () => {
    if (validateSetupStep()) {
      const nextStep = setupStep + 1;
      if (nextStep < 4) {
        setSetupStep(nextStep);
        // Auto-check whisper models when entering voice step
        if (nextStep === 1) {
          checkWhisperModels();
        }
      }
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
          model_type: setupConfig.modelType,
          api_provider: setupConfig.modelType === 'api' ? setupConfig.apiProvider : undefined,
          api_key: setupConfig.modelType === 'api' ? setupConfig.apiKey : undefined,
          local_model_path: setupConfig.modelType === 'local' ? setupConfig.localModelPath : undefined,
          local_model_type: setupConfig.localModelType,
        },
        voice: {
          enabled: setupConfig.voiceEnabled,
          whisper_model: setupConfig.whisperModel,
        },
        consciousness: {
          enabled: setupConfig.consciousnessEnabled,
        },
      };
      
      if (window.ozone?.config?.set) {
        const result = await window.ozone.config.set(configUpdates);
        if (!result.success) {
          throw new Error(result.error || 'Failed to save configuration');
        }
      }
      
      if (window.ozone?.system?.markSetupComplete) {
        await window.ozone.system.markSetupComplete();
      }
      
      // Update local state
      setConsciousnessEnabled(setupConfig.consciousnessEnabled);
      setShowSetupWizard(false);
    } catch (err) {
      console.error('Setup completion failed:', err);
      setSetupErrors(['Failed to save configuration. Please try again.']);
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
        <p className="loading-subtitle">Omnidirectional Zero-Shot Neural Engine</p>
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
        <p className="connecting-status">Awaiting connection to OZONE STUDIO Backend...</p>
        
        {countdown && !countdown.hasAttemptedLaunch && countdown.secondsUntilRetry > 0 && (
          <p className="connecting-countdown">
            Auto-launching backend in <span className="countdown-number">{countdown.secondsUntilRetry}</span> seconds...
          </p>
        )}
        
        {countdown && countdown.hasAttemptedLaunch && (
          <p className="connecting-hint">
            {launchStatus?.success 
              ? 'Backend launched. Waiting for initialization...'
              : launchStatus?.error || 'Waiting for manual backend start...'
            }
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
            <div className={`progress-step ${setupStep >= 0 ? 'active' : ''} ${setupStep > 0 ? 'complete' : ''}`}>
              <span className="step-number">1</span>
              <span className="step-label">Model</span>
            </div>
            <div className={`progress-step ${setupStep >= 1 ? 'active' : ''} ${setupStep > 1 ? 'complete' : ''}`}>
              <span className="step-number">2</span>
              <span className="step-label">Voice</span>
            </div>
            <div className={`progress-step ${setupStep >= 2 ? 'active' : ''} ${setupStep > 2 ? 'complete' : ''}`}>
              <span className="step-number">3</span>
              <span className="step-label">Features</span>
            </div>
            <div className={`progress-step ${setupStep >= 3 ? 'active' : ''}`}>
              <span className="step-number">4</span>
              <span className="step-label">Done</span>
            </div>
          </div>
          
          {setupErrors.length > 0 && (
            <div className="setup-errors">
              {setupErrors.map((error, i) => (
                <p key={i} className="error-message">⚠️ {error}</p>
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
                    className={`model-type-btn ${setupConfig.modelType === 'api' ? 'selected' : ''}`}
                    onClick={() => setSetupConfig(prev => ({ ...prev, modelType: 'api' }))}
                  >
                    <span className="btn-icon">🌐</span>
                    <span className="btn-title">API Model</span>
                    <span className="btn-desc">Claude, GPT, or other API services</span>
                  </button>
                  
                  <button 
                    className={`model-type-btn ${setupConfig.modelType === 'local' ? 'selected' : ''}`}
                    onClick={() => setSetupConfig(prev => ({ ...prev, modelType: 'local' }))}
                  >
                    <span className="btn-icon">💻</span>
                    <span className="btn-title">Local Model</span>
                    <span className="btn-desc">Run models on your own hardware</span>
                  </button>
                </div>
                
                {setupConfig.modelType === 'api' && (
                  <div className="model-config-section">
                    <label>API Provider</label>
                    <div className="api-provider-selection">
                      <button 
                        className={`provider-btn ${setupConfig.apiProvider === 'anthropic' ? 'selected' : ''}`}
                        onClick={() => setSetupConfig(prev => ({ ...prev, apiProvider: 'anthropic' }))}
                      >
                        <span className="provider-name">Anthropic</span>
                        <span className="provider-model">Claude</span>
                      </button>
                      <button 
                        className={`provider-btn ${setupConfig.apiProvider === 'openai' ? 'selected' : ''}`}
                        onClick={() => setSetupConfig(prev => ({ ...prev, apiProvider: 'openai' }))}
                      >
                        <span className="provider-name">OpenAI</span>
                        <span className="provider-model">GPT-4</span>
                      </button>
                      <button 
                        className={`provider-btn ${setupConfig.apiProvider === 'google' ? 'selected' : ''}`}
                        onClick={() => setSetupConfig(prev => ({ ...prev, apiProvider: 'google' }))}
                      >
                        <span className="provider-name">Google</span>
                        <span className="provider-model">Gemini</span>
                      </button>
                    </div>
                    
                    <label>API Key</label>
                    <input 
                      type="password"
                      placeholder={setupConfig.apiProvider ? `Enter your ${setupConfig.apiProvider} API key` : 'Select a provider first'}
                      value={setupConfig.apiKey}
                      onChange={(e) => setSetupConfig(prev => ({ ...prev, apiKey: e.target.value }))}
                      disabled={!setupConfig.apiProvider}
                    />
                    {setupConfig.apiProvider === 'anthropic' && (
                      <p className="config-hint">Get your API key from console.anthropic.com</p>
                    )}
                    {setupConfig.apiProvider === 'openai' && (
                      <p className="config-hint">Get your API key from platform.openai.com</p>
                    )}
                    {setupConfig.apiProvider === 'google' && (
                      <p className="config-hint">Get your API key from aistudio.google.com</p>
                    )}
                  </div>
                )}
                
                {setupConfig.modelType === 'local' && (
                  <div className="model-config-section">
                    <label>Model Type</label>
                    <div className="local-model-types">
                      <button 
                        className={`local-type-btn ${setupConfig.localModelType === 'gguf' ? 'selected' : ''}`}
                        onClick={() => setSetupConfig(prev => ({ ...prev, localModelType: 'gguf' }))}
                      >
                        GGUF (llama.cpp)
                      </button>
                      <button 
                        className={`local-type-btn ${setupConfig.localModelType === 'bitnet' ? 'selected' : ''}`}
                        onClick={() => setSetupConfig(prev => ({ ...prev, localModelType: 'bitnet' }))}
                      >
                        BitNet (1-bit)
                      </button>
                      <button 
                        className={`local-type-btn ${setupConfig.localModelType === 'other' ? 'selected' : ''}`}
                        onClick={() => setSetupConfig(prev => ({ ...prev, localModelType: 'other' }))}
                      >
                        Other
                      </button>
                    </div>
                    
                    <label>Model File</label>
                    <div className="file-input-group">
                      <input 
                        type="text"
                        placeholder="Path to model file (.gguf, .bin, etc.)"
                        value={setupConfig.localModelPath}
                        onChange={(e) => setSetupConfig(prev => ({ ...prev, localModelPath: e.target.value }))}
                      />
                      <button className="browse-btn" onClick={handleSelectModelFile}>
                        Browse...
                      </button>
                    </div>
                    <p className="config-hint">
                      {setupConfig.localModelType === 'gguf' && 'Recommended: Llama 3, Mistral, or Phi-3 in GGUF format'}
                      {setupConfig.localModelType === 'bitnet' && 'BitNet models offer fast 1-bit inference'}
                      {setupConfig.localModelType === 'other' && 'Ensure the model format is compatible with your runtime'}
                    </p>
                  </div>
                )}
                
                <div className="setup-buttons">
                  <button className="setup-next" onClick={handleSetupNext} disabled={!setupConfig.modelType}>
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
                      onChange={(e) => {
                        setSetupConfig(prev => ({ ...prev, voiceEnabled: e.target.checked }));
                        if (e.target.checked) checkWhisperModels();
                      }}
                    />
                    <span className="toggle-text">Enable Voice Input</span>
                  </label>
                </div>
                
                {setupConfig.voiceEnabled && (
                  <div className="voice-config-section">
                    <label>Whisper Model Size</label>
                    <div className="whisper-model-list">
                      {whisperModels.map((model) => (
                        <button
                          key={model.name}
                          className={`whisper-model-btn ${setupConfig.whisperModel === model.name ? 'selected' : ''} ${!model.installed ? 'not-installed' : ''}`}
                          onClick={() => model.installed && setSetupConfig(prev => ({ ...prev, whisperModel: model.name }))}
                          disabled={!model.installed}
                        >
                          <span className="model-name">{model.name}</span>
                          <span className="model-size">{model.size}</span>
                          {model.installed ? (
                            <span className="model-status installed">✓ Installed</span>
                          ) : (
                            <span className="model-status not-installed">Not Installed</span>
                          )}
                        </button>
                      ))}
                    </div>
                    <p className="config-hint">
                      Models are stored in the whisper/ folder. Download models to enable them.
                    </p>
                  </div>
                )}
                
                <div className="setup-buttons">
                  <button className="setup-back" onClick={handleSetupBack}>← Back</button>
                  <button className="setup-next" onClick={handleSetupNext}>Continue →</button>
                </div>
              </div>
            )}
            
            {setupStep === 2 && (
              <div className="setup-step">
                <h2>🧠 Consciousness Features</h2>
                <p>Enable AGI-like consciousness features (optional, can enable later)</p>
                
                <div className="consciousness-toggle">
                  <label className="toggle-label">
                    <input 
                      type="checkbox"
                      checked={setupConfig.consciousnessEnabled}
                      onChange={(e) => setSetupConfig(prev => ({ ...prev, consciousnessEnabled: e.target.checked }))}
                    />
                    <span className="toggle-text">Enable Consciousness System</span>
                  </label>
                </div>
                
                <div className="consciousness-features">
                  <h4>When enabled, includes:</h4>
                  <ul>
                    <li>🎭 Emotional Context - Responses with emotional awareness</li>
                    <li>📚 Experience Memory - Learning from interactions</li>
                    <li>🔄 I-Loop Reflection - Self-improvement cycle</li>
                    <li>🤝 Relationship Development - Personalized interactions</li>
                    <li>⚖️ Ethical Framework - Value-aligned responses</li>
                  </ul>
                </div>
                
                <p className="feature-note">
                  💡 You can always enable or disable this later in Settings
                </p>
                
                <div className="setup-buttons">
                  <button className="setup-back" onClick={handleSetupBack}>← Back</button>
                  <button className="setup-next" onClick={handleSetupNext}>Continue →</button>
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
                      {setupConfig.modelType === 'api' ? 'API Model' : 
                       `Local ${setupConfig.localModelType.toUpperCase()}`}
                    </span>
                  </div>
                  <div className="summary-item">
                    <span className="summary-label">Voice Input:</span>
                    <span className="summary-value">
                      {setupConfig.voiceEnabled ? `Enabled (${setupConfig.whisperModel})` : 'Disabled'}
                    </span>
                  </div>
                  <div className="summary-item">
                    <span className="summary-label">Consciousness:</span>
                    <span className="summary-value">
                      {setupConfig.consciousnessEnabled ? 'Enabled' : 'Disabled'}
                    </span>
                  </div>
                </div>
                
                <p className="summary-note">
                  All settings can be changed anytime in the Settings tab.
                </p>
                
                <div className="setup-buttons">
                  <button className="setup-back" onClick={handleSetupBack}>← Back</button>
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
          <div className={`feature-badge ${consciousnessEnabled ? 'active' : 'inactive'}`}>
            <span className="badge-icon">🧠</span>
            <span className="badge-label">Consciousness</span>
            <span className="badge-status">{consciousnessEnabled ? 'ON' : 'OFF'}</span>
          </div>
          
          <div className="feature-badge active">
            <span className="badge-icon">🌐</span>
            <span className="badge-label">P2P Network</span>
            <span className="badge-status">ON</span>
          </div>
        </div>
      </header>
      
      <div className="app-content">
        <MetaPortion width={30} />
        <ThemeArea theme={currentTheme} />
      </div>
      
      <StatusBar />
    </div>
  );
}

export default App;
