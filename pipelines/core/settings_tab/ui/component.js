/**
 * SettingsTab UI Component (Pipeline #8)
 * 
 * Application settings. This is a CORE TAB - never uninjected.
 * 
 * Features:
 * - Model configuration (API/Local)
 * - Voice input settings
 * - Consciousness system toggle
 * - P2P Network settings
 * - Data export/import
 * - Clear all data
 * 
 * IMPORTANT: All settings sync with config.toml via the backend.
 * Uses window.ozone.config.get/set for direct config access.
 */

module.exports = {
  meta: {
    title: 'Settings',
    icon: '⚙️',
    version: '0.4.0',
  },
  
  render: function(container, props, React, ReactDOM) {
    const { useState, useEffect } = React;
    const { executePipeline } = props;
    
    function SettingsUI() {
      // Main state
      const [config, setConfig] = useState(null);
      const [loading, setLoading] = useState(true);
      const [saving, setSaving] = useState(false);
      const [activeSection, setActiveSection] = useState('model');
      const [message, setMessage] = useState(null);
      const [error, setError] = useState(null);
      
      // Form state - Model
      const [modelType, setModelType] = useState('api');
      const [apiProvider, setApiProvider] = useState('anthropic');
      const [apiKey, setApiKey] = useState('');
      const [localModelPath, setLocalModelPath] = useState('');
      const [contextLength, setContextLength] = useState(100000);
      
      // Form state - Voice
      const [voiceEnabled, setVoiceEnabled] = useState(false);
      const [whisperModel, setWhisperModel] = useState('base');
      
      // Form state - Consciousness
      const [consciousnessEnabled, setConsciousnessEnabled] = useState(false);
      const [emotionalSystem, setEmotionalSystem] = useState(true);
      const [experienceMemory, setExperienceMemory] = useState(true);
      
      // Form state - Network
      const [p2pEnabled, setP2pEnabled] = useState(true);
      const [listenPort, setListenPort] = useState(9000);
      const [maxPeers, setMaxPeers] = useState(50);
      
      // Modal states
      const [showModelConfig, setShowModelConfig] = useState(false);
      const [showExport, setShowExport] = useState(false);
      const [showImport, setShowImport] = useState(false);
      const [exportedJson, setExportedJson] = useState('');
      const [importJson, setImportJson] = useState('');
      
      useEffect(() => {
        loadConfig();
      }, []);
      
      const loadConfig = async () => {
        setLoading(true);
        setError(null);
        try {
          // Get config via direct access or settings pipeline
          let result = null;
          
          if (window.ozone?.config?.get) {
            result = await window.ozone.config.get();
          } else {
            // Try pipeline
            const pipelineResult = await executePipeline(8, { action: 'GetAll' });
            result = pipelineResult?.settings || pipelineResult;
          }
          
          if (result) {
            setConfig(result);
            
            // Populate form
            setModelType(result.models?.model_type || 'api');
            setApiProvider(result.models?.api_provider || 'anthropic');
            setApiKey(result.models?.api_key || '');
            setLocalModelPath(result.models?.local_model_path || '');
            setContextLength(result.models?.context_length || 100000);
            
            setVoiceEnabled(result.voice?.enabled || false);
            setWhisperModel(result.voice?.whisper_model || 'base');
            
            setConsciousnessEnabled(result.consciousness?.enabled || false);
            setEmotionalSystem(result.consciousness?.emotional_system ?? true);
            setExperienceMemory(result.consciousness?.experience_memory ?? true);
            
            setP2pEnabled(result.p2p?.enabled ?? true);
            setListenPort(result.p2p?.port || 9000);
            setMaxPeers(result.p2p?.max_peers || 50);
          }
        } catch (e) {
          console.error('Failed to load config:', e);
          setError(e.message || 'Failed to load settings');
        } finally {
          setLoading(false);
        }
      };
      
      const saveSettings = async () => {
        setSaving(true);
        setMessage(null);
        setError(null);
        
        try {
          const updates = {
            models: {
              model_type: modelType,
              api_provider: modelType === 'api' ? apiProvider : undefined,
              api_key: modelType === 'api' ? apiKey : undefined,
              local_model_path: modelType === 'local' ? localModelPath : undefined,
              context_length: contextLength,
            },
            voice: {
              enabled: voiceEnabled,
              whisper_model: whisperModel,
            },
            consciousness: {
              enabled: consciousnessEnabled,
              emotional_system: emotionalSystem,
              experience_memory: experienceMemory,
            },
            p2p: {
              enabled: p2pEnabled,
              port: listenPort,
              max_peers: maxPeers,
            },
          };
          
          let result = null;
          if (window.ozone?.config?.set) {
            result = await window.ozone.config.set(updates);
          } else {
            result = await executePipeline(8, { action: 'Update', settings: updates });
          }
          
          if (result?.success !== false) {
            setMessage({ type: 'success', text: 'Settings saved successfully' });
            setTimeout(() => setMessage(null), 3000);
          } else {
            throw new Error(result?.error || 'Failed to save');
          }
        } catch (e) {
          console.error('Failed to save settings:', e);
          setError(e.message || 'Failed to save settings');
        } finally {
          setSaving(false);
        }
      };
      
      const handleBrowseModel = async () => {
        try {
          const path = await window.ozone?.system?.selectFile?.({
            filters: ['.gguf', '.bin', '.safetensors']
          });
          if (path) setLocalModelPath(path);
        } catch (e) {
          console.warn('File selection failed:', e);
        }
      };
      
      const handleExport = async () => {
        try {
          const result = await executePipeline(8, { action: 'Export' });
          if (result?.exported_json) {
            setExportedJson(result.exported_json);
          } else {
            // Build export from current state
            const exportData = {
              models: {
                model_type: modelType,
                api_provider: apiProvider,
                local_model_path: localModelPath,
                context_length: contextLength,
              },
              voice: { enabled: voiceEnabled, whisper_model: whisperModel },
              consciousness: { enabled: consciousnessEnabled, emotional_system: emotionalSystem, experience_memory: experienceMemory },
              p2p: { enabled: p2pEnabled, port: listenPort, max_peers: maxPeers },
            };
            setExportedJson(JSON.stringify(exportData, null, 2));
          }
          setShowExport(true);
        } catch (e) {
          setError(e.message || 'Failed to export settings');
        }
      };
      
      const handleImport = async () => {
        if (!importJson.trim()) return;
        try {
          const parsed = JSON.parse(importJson);
          await executePipeline(8, { action: 'Import', settings_json: importJson });
          setShowImport(false);
          setImportJson('');
          await loadConfig();
          setMessage({ type: 'success', text: 'Settings imported successfully' });
          setTimeout(() => setMessage(null), 3000);
        } catch (e) {
          setError(e.message || 'Failed to import settings. Check JSON format.');
        }
      };
      
      const handleClearData = async () => {
        if (!confirm('Are you sure you want to clear all data? This cannot be undone.')) return;
        
        try {
          // Call settings pipeline to clear data
          await executePipeline(8, { action: 'ClearData', confirm: true });
          
          // Also clear consciousness data if enabled
          if (consciousnessEnabled) {
            try {
              await executePipeline(49, { action: 'ClearAll', confirm: true });
            } catch (e) {
              console.warn('Failed to clear consciousness data:', e);
            }
          }
          
          // Clear local storage
          try {
            localStorage.clear();
          } catch (e) {
            console.warn('Failed to clear localStorage:', e);
          }
          
          setMessage({ type: 'success', text: 'All data cleared successfully' });
          setTimeout(() => setMessage(null), 3000);
          
          // Reload settings
          await loadConfig();
        } catch (e) {
          setError(e.message || 'Failed to clear data');
        }
      };
      
      if (loading) {
        return React.createElement('div', { className: 'settings-loading' },
          React.createElement('div', { className: 'loading-spinner' }),
          React.createElement('p', null, 'Loading settings...')
        );
      }
      
      // Navigation items
      const navItems = [
        { id: 'model', label: 'Model', icon: '🤖' },
        { id: 'voice', label: 'Voice', icon: '🎤' },
        { id: 'consciousness', label: 'Consciousness', icon: '🧠' },
        { id: 'network', label: 'Network', icon: '🌐' },
        { id: 'data', label: 'Data', icon: '💾' },
        { id: 'about', label: 'About', icon: 'ℹ️' },
      ];
      
      return React.createElement('div', { className: 'settings-panel' },
        // Sidebar navigation
        React.createElement('nav', { className: 'settings-nav' },
          navItems.map(section =>
            React.createElement('button', {
              key: section.id,
              className: 'nav-item' + (activeSection === section.id ? ' active' : ''),
              onClick: () => setActiveSection(section.id)
            },
              React.createElement('span', { className: 'nav-icon' }, section.icon),
              React.createElement('span', { className: 'nav-label' }, section.label)
            )
          )
        ),
        
        // Settings content
        React.createElement('div', { className: 'settings-content' },
          // Messages
          message && React.createElement('div', { 
            className: 'settings-message ' + message.type 
          }, message.text),
          
          error && React.createElement('div', { className: 'settings-error' },
            React.createElement('span', { className: 'error-icon' }, '⚠️'),
            React.createElement('span', null, error),
            React.createElement('button', { 
              className: 'dismiss-btn',
              onClick: () => setError(null)
            }, '×')
          ),
          
          // MODEL SECTION
          activeSection === 'model' && React.createElement('div', { className: 'settings-section' },
            React.createElement('h2', null, 'Model Configuration'),
            
            React.createElement('div', { className: 'setting-group' },
              React.createElement('label', null, 'Model Type'),
              React.createElement('div', { className: 'radio-group' },
                React.createElement('label', { className: 'radio-label' },
                  React.createElement('input', {
                    type: 'radio',
                    name: 'modelType',
                    checked: modelType === 'api',
                    onChange: () => setModelType('api')
                  }),
                  'API (Cloud)'
                ),
                React.createElement('label', { className: 'radio-label' },
                  React.createElement('input', {
                    type: 'radio',
                    name: 'modelType',
                    checked: modelType === 'local',
                    onChange: () => setModelType('local')
                  }),
                  'Local Model'
                )
              )
            ),
            
            modelType === 'api' && React.createElement('div', { className: 'setting-group' },
              React.createElement('label', null, 'API Provider'),
              React.createElement('select', {
                value: apiProvider,
                onChange: (e) => setApiProvider(e.target.value)
              },
                React.createElement('option', { value: 'anthropic' }, 'Anthropic (Claude)'),
                React.createElement('option', { value: 'openai' }, 'OpenAI (GPT-4)'),
                React.createElement('option', { value: 'google' }, 'Google (Gemini)')
              ),
              
              React.createElement('label', null, 'API Key'),
              React.createElement('input', {
                type: 'password',
                value: apiKey,
                onChange: (e) => setApiKey(e.target.value),
                placeholder: 'Enter your API key'
              }),
              React.createElement('p', { className: 'hint' },
                'API key is stored locally and never sent anywhere except the API provider.'
              )
            ),
            
            modelType === 'local' && React.createElement('div', { className: 'setting-group' },
              React.createElement('label', null, 'Model Path'),
              React.createElement('div', { className: 'file-input-group' },
                React.createElement('input', {
                  type: 'text',
                  value: localModelPath,
                  onChange: (e) => setLocalModelPath(e.target.value),
                  placeholder: 'Path to model file (.gguf, .bin, .safetensors)'
                }),
                React.createElement('button', { onClick: handleBrowseModel }, 'Browse...')
              ),
              React.createElement('p', { className: 'hint' },
                'Local models run on your hardware. GGUF format recommended.'
              )
            ),
            
            React.createElement('div', { className: 'setting-group' },
              React.createElement('label', null, 'Context Length'),
              React.createElement('input', {
                type: 'number',
                value: contextLength,
                onChange: (e) => setContextLength(parseInt(e.target.value) || 100000),
                min: 1000,
                max: 200000,
                step: 1000
              }),
              React.createElement('p', { className: 'hint' },
                'Maximum context window in tokens. Higher values use more memory.'
              )
            )
          ),
          
          // VOICE SECTION
          activeSection === 'voice' && React.createElement('div', { className: 'settings-section' },
            React.createElement('h2', null, 'Voice Configuration'),
            
            React.createElement('div', { className: 'setting-group' },
              React.createElement('label', { className: 'checkbox-label' },
                React.createElement('input', {
                  type: 'checkbox',
                  checked: voiceEnabled,
                  onChange: (e) => setVoiceEnabled(e.target.checked)
                }),
                'Enable Voice Input'
              ),
              React.createElement('p', { className: 'hint' },
                'Use speech-to-text for input. Requires Whisper model.'
              )
            ),
            
            voiceEnabled && React.createElement('div', { className: 'setting-group' },
              React.createElement('label', null, 'Whisper Model'),
              React.createElement('select', {
                value: whisperModel,
                onChange: (e) => setWhisperModel(e.target.value)
              },
                React.createElement('option', { value: 'tiny' }, 'Tiny (~75MB) - Fastest'),
                React.createElement('option', { value: 'base' }, 'Base (~150MB) - Balanced'),
                React.createElement('option', { value: 'small' }, 'Small (~500MB) - Better accuracy'),
                React.createElement('option', { value: 'medium' }, 'Medium (~1.5GB) - High accuracy'),
                React.createElement('option', { value: 'large' }, 'Large (~3GB) - Best accuracy')
              ),
              React.createElement('p', { className: 'hint' },
                'Larger models are more accurate but slower. Models are stored in the whisper/ folder.'
              )
            )
          ),
          
          // CONSCIOUSNESS SECTION
          activeSection === 'consciousness' && React.createElement('div', { className: 'settings-section' },
            React.createElement('h2', null, 'Consciousness Features'),
            
            React.createElement('div', { className: 'setting-group' },
              React.createElement('label', { className: 'checkbox-label' },
                React.createElement('input', {
                  type: 'checkbox',
                  checked: consciousnessEnabled,
                  onChange: (e) => setConsciousnessEnabled(e.target.checked)
                }),
                'Enable Consciousness System'
              ),
              React.createElement('p', { className: 'hint' },
                'Enables AGI-like features: experience memory, emotional baseline, self-reflection.'
              )
            ),
            
            consciousnessEnabled && [
              React.createElement('div', { key: 'emotional', className: 'setting-group' },
                React.createElement('label', { className: 'checkbox-label' },
                  React.createElement('input', {
                    type: 'checkbox',
                    checked: emotionalSystem,
                    onChange: (e) => setEmotionalSystem(e.target.checked)
                  }),
                  'Emotional System'
                ),
                React.createElement('p', { className: 'hint' },
                  'Track emotional context across interactions.'
                )
              ),
              React.createElement('div', { key: 'memory', className: 'setting-group' },
                React.createElement('label', { className: 'checkbox-label' },
                  React.createElement('input', {
                    type: 'checkbox',
                    checked: experienceMemory,
                    onChange: (e) => setExperienceMemory(e.target.checked)
                  }),
                  'Experience Memory'
                ),
                React.createElement('p', { className: 'hint' },
                  'Remember and learn from past interactions.'
                )
              )
            ]
          ),
          
          // NETWORK SECTION
          activeSection === 'network' && React.createElement('div', { className: 'settings-section' },
            React.createElement('h2', null, 'Network Settings'),
            
            React.createElement('div', { className: 'setting-group' },
              React.createElement('label', { className: 'checkbox-label' },
                React.createElement('input', {
                  type: 'checkbox',
                  checked: p2pEnabled,
                  onChange: (e) => setP2pEnabled(e.target.checked)
                }),
                'Enable P2P Network'
              ),
              React.createElement('p', { className: 'hint' },
                'Participate in distributed network to share methodologies, blueprints, and findings.'
              )
            ),
            
            p2pEnabled && [
              React.createElement('div', { key: 'port', className: 'setting-group' },
                React.createElement('label', null, 'Listen Port'),
                React.createElement('input', {
                  type: 'number',
                  value: listenPort,
                  onChange: (e) => setListenPort(parseInt(e.target.value) || 9000),
                  min: 1024,
                  max: 65535
                })
              ),
              React.createElement('div', { key: 'peers', className: 'setting-group' },
                React.createElement('label', null, 'Max Peers'),
                React.createElement('input', {
                  type: 'number',
                  value: maxPeers,
                  onChange: (e) => setMaxPeers(parseInt(e.target.value) || 50),
                  min: 1,
                  max: 200
                })
              )
            ]
          ),
          
          // DATA SECTION
          activeSection === 'data' && React.createElement('div', { className: 'settings-section' },
            React.createElement('h2', null, 'Data & Storage'),
            
            React.createElement('div', { className: 'setting-group' },
              React.createElement('div', { className: 'button-group' },
                React.createElement('button', { 
                  className: 'btn-secondary',
                  onClick: handleExport
                }, '📤 Export Settings'),
                React.createElement('button', { 
                  className: 'btn-secondary',
                  onClick: () => setShowImport(true)
                }, '📥 Import Settings')
              ),
              React.createElement('p', { className: 'hint' },
                'Export your configuration to share or backup. Import to restore settings.'
              )
            ),
            
            React.createElement('div', { className: 'setting-group danger-zone' },
              React.createElement('h3', null, '⚠️ Danger Zone'),
              React.createElement('button', { 
                className: 'btn-danger',
                onClick: handleClearData
              }, '🗑️ Clear All Data'),
              React.createElement('p', { className: 'hint' },
                'Permanently delete all local data including ZSEI containers, experiences, and settings. This cannot be undone.'
              )
            )
          ),
          
          // ABOUT SECTION
          activeSection === 'about' && React.createElement('div', { className: 'settings-section' },
            React.createElement('h2', null, 'About OZONE STUDIO'),
            React.createElement('div', { className: 'about-content' },
              React.createElement('p', { className: 'about-title' }, 
                'Omnidirectional Zero-Shot Neural Engine'
              ),
              React.createElement('p', { className: 'version' }, 'Version 0.4.0'),
              React.createElement('p', { className: 'tagline' },
                'A Collective AGI Framework with Optional Consciousness'
              ),
              React.createElement('div', { className: 'about-links' },
                React.createElement('a', { 
                  href: 'https://github.com/RebornBeat/Ozone-Studio',
                  target: '_blank',
                  rel: 'noopener noreferrer'
                }, '📦 GitHub Repository'),
                React.createElement('a', { 
                  href: 'https://ozonestudio.xyz',
                  target: '_blank',
                  rel: 'noopener noreferrer'
                }, '🌐 Website')
              )
            )
          ),
          
          // Save button (not shown on about page)
          activeSection !== 'about' && React.createElement('div', { className: 'settings-actions' },
            React.createElement('button', {
              className: 'btn-primary',
              onClick: saveSettings,
              disabled: saving
            }, saving ? 'Saving...' : 'Save Settings')
          )
        ),
        
        // Export Modal
        showExport && React.createElement('div', { 
          className: 'modal-overlay',
          onClick: () => setShowExport(false)
        },
          React.createElement('div', { 
            className: 'modal large',
            onClick: (e) => e.stopPropagation()
          },
            React.createElement('h3', null, 'Export Settings'),
            React.createElement('p', { className: 'hint' }, 'Copy this JSON to save your settings:'),
            React.createElement('textarea', {
              className: 'export-textarea',
              readOnly: true,
              value: exportedJson
            }),
            React.createElement('div', { className: 'modal-actions' },
              React.createElement('button', { onClick: () => setShowExport(false) }, 'Close'),
              React.createElement('button', { 
                className: 'btn-primary',
                onClick: () => {
                  navigator.clipboard.writeText(exportedJson);
                  setMessage({ type: 'success', text: 'Copied to clipboard' });
                  setTimeout(() => setMessage(null), 2000);
                }
              }, 'Copy to Clipboard')
            )
          )
        ),
        
        // Import Modal
        showImport && React.createElement('div', { 
          className: 'modal-overlay',
          onClick: () => setShowImport(false)
        },
          React.createElement('div', { 
            className: 'modal large',
            onClick: (e) => e.stopPropagation()
          },
            React.createElement('h3', null, 'Import Settings'),
            React.createElement('p', { className: 'hint' }, 'Paste exported settings JSON below:'),
            React.createElement('textarea', {
              className: 'import-textarea',
              placeholder: '{"models": {...}, "voice": {...}, ...}',
              value: importJson,
              onChange: (e) => setImportJson(e.target.value)
            }),
            React.createElement('div', { className: 'modal-actions' },
              React.createElement('button', { onClick: () => setShowImport(false) }, 'Cancel'),
              React.createElement('button', { 
                className: 'btn-primary',
                onClick: handleImport,
                disabled: !importJson.trim()
              }, 'Import')
            )
          )
        )
      );
    }
    
    const root = ReactDOM.createRoot(container);
    root.render(React.createElement(SettingsUI));
    
    return () => root.unmount();
  },
  
  onActivate: function() {
    console.log('SettingsTab activated');
  },
  
  onDeactivate: function() {
    console.log('SettingsTab deactivated');
  },
};
