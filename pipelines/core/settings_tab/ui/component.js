/**
 * SettingsTab UI Component (Pipeline #8)
 * 
 * Application settings. This is a CORE TAB.
 * 
 * IMPORTANT: All settings sync with config.toml via the backend.
 * The SettingsTab pipeline reads/writes config.toml.
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
      const [config, setConfig] = useState(null);
      const [loading, setLoading] = useState(true);
      const [saving, setSaving] = useState(false);
      const [activeSection, setActiveSection] = useState('model');
      const [message, setMessage] = useState(null);
      
      // Form state
      const [modelType, setModelType] = useState('api');
      const [apiProvider, setApiProvider] = useState('anthropic');
      const [apiKey, setApiKey] = useState('');
      const [localModelPath, setLocalModelPath] = useState('');
      const [voiceEnabled, setVoiceEnabled] = useState(false);
      const [whisperModel, setWhisperModel] = useState('base');
      const [consciousnessEnabled, setConsciousnessEnabled] = useState(false);
      const [p2pEnabled, setP2pEnabled] = useState(true);
      
      useEffect(() => {
        loadConfig();
      }, []);
      
      const loadConfig = async () => {
        setLoading(true);
        try {
          // Get config via settings pipeline
          const result = await window.ozone?.config?.get?.();
          if (result) {
            setConfig(result);
            // Populate form
            setModelType(result.models?.model_type || 'api');
            setApiProvider(result.models?.api_provider || 'anthropic');
            setApiKey(result.models?.api_key || '');
            setLocalModelPath(result.models?.local_model_path || '');
            setVoiceEnabled(result.voice?.enabled || false);
            setWhisperModel(result.voice?.whisper_model || 'base');
            setConsciousnessEnabled(result.consciousness?.enabled || false);
            setP2pEnabled(result.p2p?.enabled ?? true);
          }
        } catch (e) {
          console.error('Failed to load config:', e);
          setMessage({ type: 'error', text: 'Failed to load settings' });
        } finally {
          setLoading(false);
        }
      };
      
      const saveSettings = async () => {
        setSaving(true);
        setMessage(null);
        try {
          const updates = {
            models: {
              model_type: modelType,
              api_provider: modelType === 'api' ? apiProvider : undefined,
              api_key: modelType === 'api' ? apiKey : undefined,
              local_model_path: modelType === 'local' ? localModelPath : undefined,
            },
            voice: {
              enabled: voiceEnabled,
              whisper_model: whisperModel,
            },
            consciousness: {
              enabled: consciousnessEnabled,
            },
            p2p: {
              enabled: p2pEnabled,
            },
          };
          
          const result = await window.ozone?.config?.set?.(updates);
          if (result?.success) {
            setMessage({ type: 'success', text: 'Settings saved successfully' });
          } else {
            throw new Error(result?.error || 'Failed to save');
          }
        } catch (e) {
          console.error('Failed to save settings:', e);
          setMessage({ type: 'error', text: 'Failed to save settings' });
        } finally {
          setSaving(false);
        }
      };
      
      const handleBrowseModel = async () => {
        const path = await window.ozone?.system?.selectFile?.({
          filters: ['.gguf', '.bin', '.safetensors']
        });
        if (path) setLocalModelPath(path);
      };
      
      if (loading) {
        return React.createElement('div', { className: 'settings-loading' },
          React.createElement('div', { className: 'loading-spinner' }),
          React.createElement('p', null, 'Loading settings...')
        );
      }
      
      return React.createElement('div', { className: 'settings-panel' },
        // Sidebar navigation
        React.createElement('nav', { className: 'settings-nav' },
          [
            { id: 'model', label: 'Model', icon: '🤖' },
            { id: 'voice', label: 'Voice', icon: '🎤' },
            { id: 'consciousness', label: 'Consciousness', icon: '🧠' },
            { id: 'network', label: 'Network', icon: '🌐' },
            { id: 'about', label: 'About', icon: 'ℹ️' },
          ].map(section =>
            React.createElement('button', {
              key: section.id,
              className: `nav-item ${activeSection === section.id ? 'active' : ''}`,
              onClick: () => setActiveSection(section.id)
            },
              React.createElement('span', { className: 'nav-icon' }, section.icon),
              React.createElement('span', { className: 'nav-label' }, section.label)
            )
          )
        ),
        
        // Settings content
        React.createElement('div', { className: 'settings-content' },
          // Message
          message && React.createElement('div', { 
            className: `settings-message ${message.type}` 
          }, message.text),
          
          // Model settings
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
              })
            ),
            
            modelType === 'local' && React.createElement('div', { className: 'setting-group' },
              React.createElement('label', null, 'Model Path'),
              React.createElement('div', { className: 'file-input-group' },
                React.createElement('input', {
                  type: 'text',
                  value: localModelPath,
                  onChange: (e) => setLocalModelPath(e.target.value),
                  placeholder: 'Path to model file'
                }),
                React.createElement('button', { onClick: handleBrowseModel }, 'Browse...')
              )
            )
          ),
          
          // Voice settings
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
              )
            ),
            
            voiceEnabled && React.createElement('div', { className: 'setting-group' },
              React.createElement('label', null, 'Whisper Model'),
              React.createElement('select', {
                value: whisperModel,
                onChange: (e) => setWhisperModel(e.target.value)
              },
                React.createElement('option', { value: 'tiny' }, 'Tiny (~75MB)'),
                React.createElement('option', { value: 'base' }, 'Base (~150MB)'),
                React.createElement('option', { value: 'small' }, 'Small (~500MB)'),
                React.createElement('option', { value: 'medium' }, 'Medium (~1.5GB)'),
                React.createElement('option', { value: 'large' }, 'Large (~3GB)')
              ),
              React.createElement('p', { className: 'hint' },
                'Models are stored in the whisper/ folder.'
              )
            )
          ),
          
          // Consciousness settings
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
                'Enables experience memory, emotional baseline, I-Loop self-reflection, and other AGI-like features.'
              )
            )
          ),
          
          // Network settings
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
                'Participate in the distributed network to share and discover methodologies, blueprints, and findings.'
              )
            )
          ),
          
          // About
          activeSection === 'about' && React.createElement('div', { className: 'settings-section' },
            React.createElement('h2', null, 'About OZONE STUDIO'),
            React.createElement('p', null, 
              'Omnidirectional Zero-Shot Neural Engine'
            ),
            React.createElement('p', { className: 'version' }, 'Version 0.4.0'),
            React.createElement('p', { className: 'hint' },
              'A Collective AGI Framework with Optional Consciousness'
            )
          ),
          
          // Save button
          activeSection !== 'about' && React.createElement('div', { className: 'settings-actions' },
            React.createElement('button', {
              className: 'btn-primary',
              onClick: saveSettings,
              disabled: saving
            }, saving ? 'Saving...' : 'Save Settings')
          )
        )
      );
    }
    
    const root = ReactDOM.createRoot(container);
    root.render(React.createElement(SettingsUI));
    
    return () => root.unmount();
  },
};
