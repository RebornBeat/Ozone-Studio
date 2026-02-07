/**
 * SettingsTab UI Component
 * Pipeline #8 - SettingsTabPipeline
 * 
 * Manages all system configuration:
 * - Model selection (API/Local)
 * - Voice input
 * - Consciousness system
 * - P2P Network
 * - Data & Privacy
 * 
 * Reads/writes config.toml via gRPC:
 * - window.ozone.pipeline.execute(8, input) for settings pipeline
 * - window.ozone.config.get() / .set() for direct config access
 */

import React, { useEffect, useState } from 'react';

interface ModelSettings {
  active_model_type: string;
  active_model_identifier: string;
  api_provider?: string;
  api_endpoint?: string;
  api_key_env: string;
  local_model_path?: string;
  context_length: number;
  gpu_layers?: number;
  allow_user_selection: boolean;
  available_models: AvailableModel[];
}

interface AvailableModel {
  name: string;
  model_type: string;
  identifier: string;
  description?: string;
  context_length?: number;
  is_local: boolean;
}

interface UISettings {
  theme: string;
  meta_portion_width_percent: number;
  show_task_recommendations: boolean;
  show_connection_bar: boolean;
  default_tabs: string[];
}

interface NetworkSettings {
  enabled: boolean;
  listen_port: number;
  max_peers: number;
  bootstrap_nodes: string[];
}

interface ConsciousnessSettings {
  enabled: boolean;
  emotional_system_enabled: boolean;
  experience_memory_enabled: boolean;
  show_emotional_state: boolean;
  show_decision_reasoning: boolean;
  playback_enabled: boolean;
}

interface VoiceSettings {
  enabled: boolean;
  whisper_model: string;
}

interface AllSettings {
  models: ModelSettings;
  ui: UISettings;
  network: NetworkSettings;
  consciousness?: ConsciousnessSettings;
  voice?: VoiceSettings;
}

export function SettingsTab() {
  const [settings, setSettings] = useState<AllSettings | null>(null);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);
  
  // Modal states
  const [showAddModel, setShowAddModel] = useState(false);
  const [showExport, setShowExport] = useState(false);
  const [showImport, setShowImport] = useState(false);
  const [exportedJson, setExportedJson] = useState('');
  const [importJson, setImportJson] = useState('');

  useEffect(() => {
    loadSettings();
  }, []);

  const executePipeline = async (pipelineId: number, input: any) => {
    if (window.ozone?.pipeline?.execute) {
      return await window.ozone.pipeline.execute(pipelineId, input);
    }
    throw new Error('Pipeline execution not available');
  };

  const loadSettings = async () => {
    setLoading(true);
    setError(null);
    try {
      // Try SettingsTabPipeline first
      const result = await executePipeline(8, { action: 'GetAll' });
      if (result?.settings) {
        setSettings(result.settings);
      } else {
        // Fallback to direct config
        if (window.ozone?.config?.get) {
          const config = await window.ozone.config.get();
          // Map config to settings structure
          setSettings({
            models: {
              active_model_type: config.models?.model_type || 'api',
              active_model_identifier: config.models?.api_key ? 'configured' : 'none',
              api_provider: config.models?.api_provider,
              api_key_env: 'OZONE_API_KEY',
              local_model_path: config.models?.local_model_path,
              context_length: 100000,
              allow_user_selection: true,
              available_models: [],
            },
            ui: {
              theme: config.ui?.theme || 'home_dashboard',
              meta_portion_width_percent: config.ui?.meta_portion_width || 30,
              show_task_recommendations: true,
              show_connection_bar: true,
              default_tabs: ['workspace', 'tasks', 'library', 'settings'],
            },
            network: {
              enabled: config.p2p?.enabled || false,
              listen_port: config.p2p?.port || 9000,
              max_peers: 50,
              bootstrap_nodes: [],
            },
            consciousness: {
              enabled: config.consciousness?.enabled || false,
              emotional_system_enabled: config.consciousness?.emotional_system || true,
              experience_memory_enabled: config.consciousness?.experience_memory || true,
              show_emotional_state: true,
              show_decision_reasoning: false,
              playback_enabled: false,
            },
            voice: {
              enabled: config.voice?.enabled || false,
              whisper_model: config.voice?.whisper_model || 'base',
            },
          });
        }
      }
    } catch (e: any) {
      setError(e.message || 'Failed to load settings');
    }
    setLoading(false);
  };

  const saveSettings = async (updates: Partial<AllSettings>) => {
    setSaving(true);
    setError(null);
    try {
      // Convert to config format and save
      const configUpdates: any = {};
      
      if (updates.models) {
        configUpdates.models = {
          model_type: updates.models.active_model_type,
          api_provider: updates.models.api_provider,
          local_model_path: updates.models.local_model_path,
        };
      }
      
      if (updates.consciousness) {
        configUpdates.consciousness = {
          enabled: updates.consciousness.enabled,
          emotional_system: updates.consciousness.emotional_system_enabled,
          experience_memory: updates.consciousness.experience_memory_enabled,
        };
      }
      
      if (updates.voice) {
        configUpdates.voice = {
          enabled: updates.voice.enabled,
          whisper_model: updates.voice.whisper_model,
        };
      }
      
      if (updates.network) {
        configUpdates.p2p = {
          enabled: updates.network.enabled,
          port: updates.network.listen_port,
        };
      }
      
      if (updates.ui) {
        configUpdates.ui = {
          theme: updates.ui.theme,
          meta_portion_width: updates.ui.meta_portion_width_percent,
        };
      }
      
      if (window.ozone?.config?.set) {
        const result = await window.ozone.config.set(configUpdates);
        if (result?.success) {
          setSuccessMessage('Settings saved');
          setTimeout(() => setSuccessMessage(null), 3000);
          // Reload settings
          await loadSettings();
        } else {
          throw new Error(result?.error || 'Failed to save');
        }
      }
    } catch (e: any) {
      setError(e.message || 'Failed to save settings');
    }
    setSaving(false);
  };

  const handleToggle = async (section: string, key: string, value: boolean) => {
    if (!settings) return;
    
    const updates: any = {};
    if (section === 'consciousness') {
      updates.consciousness = { ...(settings.consciousness || {}), [key]: value };
    } else if (section === 'voice') {
      updates.voice = { ...(settings.voice || {}), [key]: value };
    } else if (section === 'network') {
      updates.network = { ...settings.network, [key]: value };
    }
    
    // Update local state immediately
    setSettings(prev => prev ? {
      ...prev,
      [section]: { ...((prev as any)[section] || {}), [key]: value }
    } : null);
    
    await saveSettings(updates);
  };

  const handleExport = async () => {
    try {
      const result = await executePipeline(8, { action: 'Export' });
      if (result?.exported_json) {
        setExportedJson(result.exported_json);
        setShowExport(true);
      }
    } catch (e: any) {
      setError(e.message || 'Failed to export settings');
    }
  };

  const handleImport = async () => {
    if (!importJson.trim()) return;
    try {
      await executePipeline(8, { action: 'Import', settings_json: importJson });
      setShowImport(false);
      setImportJson('');
      await loadSettings();
      setSuccessMessage('Settings imported successfully');
      setTimeout(() => setSuccessMessage(null), 3000);
    } catch (e: any) {
      setError(e.message || 'Failed to import settings');
    }
  };

  const handleClearData = async () => {
    if (!confirm('Are you sure you want to clear all data? This cannot be undone.')) return;
    
    try {
      // Call settings pipeline to clear data
      await (window as any).ozone?.pipeline?.execute(8, {
        action: 'ClearData',
        confirm: true,
      });
      
      // Also clear consciousness data if enabled
      if (settings.consciousness_enabled) {
        await (window as any).ozone?.pipeline?.execute(49, {
          action: 'ClearAll',
          confirm: true,
        });
      }
      
      // Clear local storage
      localStorage.clear();
      
      setSuccessMessage('All data cleared successfully');
      setTimeout(() => setSuccessMessage(null), 3000);
      
      // Reload settings
      await loadSettings();
    } catch (e: any) {
      setError(e.message || 'Failed to clear data');
      setTimeout(() => setError(null), 5000);
    }
  };

  if (loading) {
    return (
      <div className="settings-tab">
        <div className="panel-loading">
          <div className="loading-spinner" />
          <p>Loading settings...</p>
        </div>
      </div>
    );
  }

  if (!settings) {
    return (
      <div className="settings-tab">
        <div className="panel-error">
          <span className="error-icon">⚠️</span>
          <p>Failed to load settings</p>
          <button onClick={loadSettings}>Retry</button>
        </div>
      </div>
    );
  }

  return (
    <div className="settings-tab">
      <div className="panel-toolbar">
        <h2>Settings</h2>
        {saving && <span className="saving-indicator">Saving...</span>}
        {successMessage && <span className="success-indicator">{successMessage}</span>}
      </div>

      {error && (
        <div className="panel-error">
          <span className="error-icon">⚠️</span>
          <p>{error}</p>
          <button onClick={() => setError(null)}>Dismiss</button>
        </div>
      )}

      <div className="settings-sections">
        {/* Model Configuration */}
        <div className="settings-card">
          <div className="card-header">
            <span className="card-icon">🤖</span>
            <h3>Model Configuration</h3>
          </div>
          <div className="card-body">
            <div className="setting-row">
              <span className="setting-label">Model Type</span>
              <span className="setting-value">{settings.models.active_model_type}</span>
            </div>
            {settings.models.api_provider && (
              <div className="setting-row">
                <span className="setting-label">Provider</span>
                <span className="setting-value">{settings.models.api_provider}</span>
              </div>
            )}
            {settings.models.local_model_path && (
              <div className="setting-row">
                <span className="setting-label">Model Path</span>
                <span className="setting-value truncate">{settings.models.local_model_path}</span>
              </div>
            )}
            <div className="setting-row">
              <span className="setting-label">Context Length</span>
              <span className="setting-value">{settings.models.context_length.toLocaleString()} tokens</span>
            </div>
            <button className="card-btn" onClick={() => setShowAddModel(true)}>
              Configure Models
            </button>
          </div>
        </div>

        {/* Voice Configuration */}
        <div className="settings-card">
          <div className="card-header">
            <span className="card-icon">🎤</span>
            <h3>Voice Input</h3>
          </div>
          <div className="card-body">
            <div className="setting-row">
              <span className="setting-label">Enable Voice</span>
              <label className="toggle-switch">
                <input 
                  type="checkbox"
                  checked={settings.voice?.enabled || false}
                  onChange={(e) => handleToggle('voice', 'enabled', e.target.checked)}
                  disabled={saving}
                />
                <span className="toggle-slider" />
              </label>
            </div>
            <div className="setting-row">
              <span className="setting-label">Whisper Model</span>
              <span className="setting-value">{settings.voice?.whisper_model || 'base'}</span>
            </div>
            <p className="setting-hint">
              Voice input uses Whisper for speech-to-text
            </p>
          </div>
        </div>

        {/* Consciousness */}
        <div className="settings-card">
          <div className="card-header">
            <span className="card-icon">🧠</span>
            <h3>Consciousness System</h3>
          </div>
          <div className="card-body">
            <div className="setting-row">
              <span className="setting-label">Enable Consciousness</span>
              <label className="toggle-switch">
                <input 
                  type="checkbox"
                  checked={settings.consciousness?.enabled || false}
                  onChange={(e) => handleToggle('consciousness', 'enabled', e.target.checked)}
                  disabled={saving}
                />
                <span className="toggle-slider" />
              </label>
            </div>
            {settings.consciousness?.enabled && (
              <>
                <div className="setting-row">
                  <span className="setting-label">Emotional System</span>
                  <label className="toggle-switch">
                    <input 
                      type="checkbox"
                      checked={settings.consciousness?.emotional_system_enabled || false}
                      onChange={(e) => handleToggle('consciousness', 'emotional_system_enabled', e.target.checked)}
                      disabled={saving}
                    />
                    <span className="toggle-slider" />
                  </label>
                </div>
                <div className="setting-row">
                  <span className="setting-label">Experience Memory</span>
                  <label className="toggle-switch">
                    <input 
                      type="checkbox"
                      checked={settings.consciousness?.experience_memory_enabled || false}
                      onChange={(e) => handleToggle('consciousness', 'experience_memory_enabled', e.target.checked)}
                      disabled={saving}
                    />
                    <span className="toggle-slider" />
                  </label>
                </div>
              </>
            )}
            <p className="setting-hint">
              Emotional context, experience memory, and self-reflection
            </p>
          </div>
        </div>

        {/* P2P Network */}
        <div className="settings-card">
          <div className="card-header">
            <span className="card-icon">🌐</span>
            <h3>P2P Network</h3>
          </div>
          <div className="card-body">
            <div className="setting-row">
              <span className="setting-label">Enable P2P</span>
              <label className="toggle-switch">
                <input 
                  type="checkbox"
                  checked={settings.network.enabled}
                  onChange={(e) => handleToggle('network', 'enabled', e.target.checked)}
                  disabled={saving}
                />
                <span className="toggle-slider" />
              </label>
            </div>
            <div className="setting-row">
              <span className="setting-label">Listen Port</span>
              <span className="setting-value">{settings.network.listen_port}</span>
            </div>
            <div className="setting-row">
              <span className="setting-label">Max Peers</span>
              <span className="setting-value">{settings.network.max_peers}</span>
            </div>
            <p className="setting-hint">
              Share and discover methodologies, blueprints, and findings
            </p>
          </div>
        </div>

        {/* Data & Storage */}
        <div className="settings-card">
          <div className="card-header">
            <span className="card-icon">💾</span>
            <h3>Data & Storage</h3>
          </div>
          <div className="card-body">
            <button className="card-btn" onClick={handleExport}>
              Export Settings
            </button>
            <button className="card-btn" onClick={() => setShowImport(true)}>
              Import Settings
            </button>
            <button className="card-btn danger" onClick={handleClearData}>
              Clear All Data
            </button>
            <p className="setting-hint">
              Export/import your configuration. Clear data removes all local storage.
            </p>
          </div>
        </div>
      </div>

      {/* Add Model Modal */}
      {showAddModel && (
        <div className="dialog-overlay" onClick={() => setShowAddModel(false)}>
          <div className="dialog large" onClick={e => e.stopPropagation()}>
            <h3>Configure Models</h3>
            <p className="dialog-hint">
              Configure API credentials or local model paths.
            </p>
            
            <div className="model-config-form">
              <div className="form-section">
                <h4>API Configuration</h4>
                <div className="form-row">
                  <label>Provider</label>
                  <select defaultValue={settings.models.api_provider || ''}>
                    <option value="">Select provider...</option>
                    <option value="anthropic">Anthropic (Claude)</option>
                    <option value="openai">OpenAI (GPT)</option>
                    <option value="google">Google (Gemini)</option>
                  </select>
                </div>
                <div className="form-row">
                  <label>API Key</label>
                  <input type="password" placeholder="Enter API key" />
                </div>
              </div>
              
              <div className="form-section">
                <h4>Local Model</h4>
                <div className="form-row">
                  <label>Model Path</label>
                  <div className="file-input-group">
                    <input 
                      type="text" 
                      id="local-model-path-input"
                      placeholder="/path/to/model.gguf"
                      defaultValue={settings.models.local_model_path || ''}
                      onChange={(e) => {
                        setSettings(prev => ({
                          ...prev,
                          models: { ...prev.models, local_model_path: e.target.value }
                        }));
                      }}
                    />
                    <button onClick={async () => {
                      try {
                        const path = await window.ozone?.system?.selectFile?.({
                          filters: ['.gguf', '.bin', '.safetensors']
                        });
                        if (path) {
                          const input = document.getElementById('local-model-path-input') as HTMLInputElement;
                          if (input) {
                            input.value = path;
                          }
                          setSettings(prev => ({
                            ...prev,
                            models: { ...prev.models, local_model_path: path }
                          }));
                        }
                      } catch (e) {
                        console.warn('File selection failed:', e);
                      }
                    }}>Browse</button>
                  </div>
                </div>
              </div>
            </div>
            
            <div className="dialog-buttons">
              <button className="btn-cancel" onClick={() => setShowAddModel(false)}>Cancel</button>
              <button className="btn-confirm">Save</button>
            </div>
          </div>
        </div>
      )}

      {/* Export Modal */}
      {showExport && (
        <div className="dialog-overlay" onClick={() => setShowExport(false)}>
          <div className="dialog large" onClick={e => e.stopPropagation()}>
            <h3>Export Settings</h3>
            <textarea 
              className="export-textarea"
              readOnly
              value={exportedJson}
            />
            <div className="dialog-buttons">
              <button className="btn-cancel" onClick={() => setShowExport(false)}>Close</button>
              <button className="btn-confirm" onClick={() => {
                navigator.clipboard.writeText(exportedJson);
                setSuccessMessage('Copied to clipboard');
                setTimeout(() => setSuccessMessage(null), 2000);
              }}>Copy</button>
            </div>
          </div>
        </div>
      )}

      {/* Import Modal */}
      {showImport && (
        <div className="dialog-overlay" onClick={() => setShowImport(false)}>
          <div className="dialog large" onClick={e => e.stopPropagation()}>
            <h3>Import Settings</h3>
            <p className="dialog-hint">Paste exported settings JSON below:</p>
            <textarea 
              className="import-textarea"
              placeholder='{"models": {...}, "ui": {...}}'
              value={importJson}
              onChange={(e) => setImportJson(e.target.value)}
            />
            <div className="dialog-buttons">
              <button className="btn-cancel" onClick={() => setShowImport(false)}>Cancel</button>
              <button className="btn-confirm" onClick={handleImport} disabled={!importJson.trim()}>
                Import
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default SettingsTab;
