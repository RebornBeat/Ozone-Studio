/**
 * Ozone Studio - Electron Preload Script
 * 
 * Exposes a safe API to the renderer process for communicating
 * with the Rust backend via the main process.
 */

const { contextBridge, ipcRenderer } = require('electron');

// Expose the Ozone API to the renderer
contextBridge.exposeInMainWorld('ozone', {
  // ========================================================================
  // Authentication
  // ========================================================================
  auth: {
    /**
     * Request an authentication challenge
     * @param {Uint8Array} publicKey - User's public key
     * @returns {Promise<{challenge: Uint8Array}>}
     */
    challenge: (publicKey) => ipcRenderer.invoke('auth:challenge', publicKey),
    
    /**
     * Authenticate with signed challenge
     * @param {Uint8Array} publicKey 
     * @param {Uint8Array} signature 
     * @returns {Promise<{success: boolean, sessionToken?: Uint8Array, error?: string}>}
     */
    authenticate: (publicKey, signature) => 
      ipcRenderer.invoke('auth:authenticate', { publicKey, signature }),
  },
  
  // ========================================================================
  // Pipeline Execution
  // ========================================================================
  pipeline: {
    /**
     * Execute a pipeline
     * @param {number} pipelineId 
     * @param {object} input 
     * @returns {Promise<{success: boolean, taskId: number, output?: object, error?: string}>}
     */
    execute: (pipelineId, input) => 
      ipcRenderer.invoke('pipeline:execute', { pipelineId, input }),
    
    /**
     * List available pipelines
     * @returns {Promise<{pipelines: Array<{id: number, name: string}>}>}
     */
    list: () => ipcRenderer.invoke('pipeline:list'),
    
    /**
     * Get pipeline blueprint/metadata
     * @param {number} pipelineId 
     * @returns {Promise<object>}
     */
    getBlueprint: (pipelineId) => 
      ipcRenderer.invoke('pipeline:blueprint', pipelineId),
    
    /**
     * v0.4.0 - Get full pipeline registry (THE SINGLE SOURCE OF TRUTH)
     * @returns {Promise<{success: boolean, registry: Array<{id, name, folder_name, category, has_ui, is_tab, description}>}>}
     */
    getRegistry: () => ipcRenderer.invoke('pipeline:registry'),
    
    /**
     * v0.4.0 - Get pipeline UI component.js content
     * @param {number} pipelineId 
     * @returns {Promise<{success: boolean, component_js?: string, error?: string}>}
     */
    getUIComponent: (pipelineId) => ipcRenderer.invoke('pipeline:ui-component', pipelineId),
  },
  
  // ========================================================================
  // ZSEI Queries
  // ========================================================================
  zsei: {
    /**
     * Execute a ZSEI query
     * @param {object} query 
     * @returns {Promise<object>}
     */
    query: (query) => ipcRenderer.invoke('zsei:query', query),
    
    /**
     * Traverse ZSEI from a starting container
     * @param {object} request 
     * @returns {Promise<object>}
     */
    traverse: (request) => ipcRenderer.invoke('zsei:traverse', request),
    
    /**
     * Get a container by ID
     * @param {number} containerId 
     * @returns {Promise<object>}
     */
    getContainer: (containerId) => 
      ipcRenderer.invoke('zsei:container', containerId),
  },
  
  // ========================================================================
  // Task Management
  // ========================================================================
  task: {
    /**
     * Get task status
     * @param {number} taskId 
     * @returns {Promise<object>}
     */
    status: (taskId) => ipcRenderer.invoke('task:status', taskId),
    
    /**
     * List active tasks
     * @returns {Promise<{tasks: Array}>}
     */
    list: () => ipcRenderer.invoke('task:list'),
    
    /**
     * Cancel a task
     * @param {number} taskId 
     * @returns {Promise<{success: boolean}>}
     */
    cancel: (taskId) => ipcRenderer.invoke('task:cancel', taskId),
    
    /**
     * Subscribe to task updates
     * @param {number} taskId 
     * @param {function} callback 
     */
    subscribe: (taskId, callback) => {
      const handler = (event, data) => {
        if (data.taskId === taskId) {
          callback(data);
        }
      };
      ipcRenderer.on('task:update', handler);
      return () => ipcRenderer.removeListener('task:update', handler);
    },
  },
  
  // ========================================================================
  // Configuration
  // ========================================================================
  config: {
    /**
     * Get current configuration
     * @returns {Promise<object>}
     */
    get: () => ipcRenderer.invoke('config:get'),
    
    /**
     * Update configuration
     * @param {object} updates 
     * @returns {Promise<{success: boolean}>}
     */
    set: (updates) => ipcRenderer.invoke('config:set', updates),
  },
  
  // ========================================================================
  // System
  // ========================================================================
  system: {
    /**
     * Get system stats
     * @returns {Promise<object>}
     */
    getStats: () => ipcRenderer.invoke('system:getStats'),
    
    /**
     * Check if this is first launch (needs setup wizard)
     * @returns {Promise<boolean>}
     */
    isFirstLaunch: () => ipcRenderer.invoke('system:isFirstLaunch'),
    
    /**
     * Mark setup as complete
     * @returns {Promise<{success: boolean}>}
     */
    markSetupComplete: () => ipcRenderer.invoke('system:markSetupComplete'),
    
    /**
     * Open file selection dialog
     * @param {object} options - {filters?: string[]}
     * @returns {Promise<string|null>}
     */
    selectFile: (options) => ipcRenderer.invoke('system:selectFile', options),
    
    /**
     * Check which whisper models are installed
     * @returns {Promise<string[]>} - Array of installed model names
     */
    checkWhisperModels: () => ipcRenderer.invoke('system:checkWhisperModels'),
  },
  
  // ========================================================================
  // UI Events
  // ========================================================================
  events: {
    /**
     * Subscribe to backend errors
     * @param {function} callback 
     */
    onBackendError: (callback) => {
      ipcRenderer.on('backend-error', (event, data) => callback(data));
    },
    
    /**
     * Subscribe to connection status changes
     * @param {function} callback 
     */
    onConnectionChange: (callback) => {
      ipcRenderer.on('connection-change', (event, data) => callback(data));
    },
    
    /**
     * Subscribe to connection countdown (before auto-launch)
     * @param {function} callback - {secondsUntilRetry, willAutoLaunch, hasAttemptedLaunch}
     */
    onConnectionCountdown: (callback) => {
      ipcRenderer.on('connection-countdown', (event, data) => callback(data));
    },
    
    /**
     * Subscribe to backend launch status
     * @param {function} callback - {success, message?, error?}
     */
    onBackendLaunchStatus: (callback) => {
      ipcRenderer.on('backend-launch-status', (event, data) => callback(data));
    },
    
    /**
     * Subscribe to system stats updates
     * @param {function} callback
     */
    onStatsUpdate: (callback) => {
      ipcRenderer.on('stats-update', (event, data) => callback(data));
    },
  },
});

// Log that preload is ready
console.log('Ozone Studio preload script loaded');
