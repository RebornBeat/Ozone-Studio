/**
 * Ozone Studio - Main React Application
 * 
 * Based on Section 5 & 27 of the specification.
 * 
 * The UI works WITH pipelines, not replacing them:
 * - ThemePipeline controls the main content area
 * - MetaPortion is always accessible (prompt, voice, tasks, home)
 * - ConnectionBar shows network status
 */

import React, { useEffect, useState } from 'react';
import { useOzoneStore } from './services/store';
import { MetaPortion } from './components/MetaPortion';
import { ThemeArea } from './components/ThemeArea';
import { ConnectionBar } from './components/ConnectionBar';
import './App.css';

// TypeScript declaration for the ozone API exposed by preload
declare global {
  interface Window {
    ozone: {
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
      events: {
        onBackendError: (callback: (data: object) => void) => void;
        onConnectionChange: (callback: (data: object) => void) => void;
      };
    };
  }
}

/**
 * Main Application Component
 * 
 * Layout (§5.1):
 * ┌─────────────────────────────────────────────────────────────┐
 * │                     Connection Bar                          │
 * ├───────────────┬─────────────────────────────────────────────┤
 * │               │                                             │
 * │  Meta Portion │              Theme Area                     │
 * │  (20% width)  │         (Pipeline-driven UI)                │
 * │               │                                             │
 * │  - Prompt     │                                             │
 * │  - Voice      │                                             │
 * │  - Tasks      │                                             │
 * │  - Home       │                                             │
 * │               │                                             │
 * └───────────────┴─────────────────────────────────────────────┘
 */
function App() {
  const { 
    isConnected, 
    isAuthenticated, 
    currentTheme,
    metaPortionWidth,
    initializeApp,
  } = useOzoneStore();
  
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Initialize the application
    async function init() {
      try {
        setLoading(true);
        
        // Load configuration
        const config = await window.ozone.config.get();
        
        // Initialize store with config
        await initializeApp(config);
        
        // Subscribe to backend errors
        window.ozone.events.onBackendError((data: any) => {
          setError(`Backend error: ${data.code}`);
        });
        
        // Subscribe to connection changes
        window.ozone.events.onConnectionChange((data: any) => {
          console.log('Connection changed:', data);
        });
        
        setLoading(false);
      } catch (err) {
        console.error('Failed to initialize:', err);
        setError('Failed to connect to Ozone Studio backend');
        setLoading(false);
      }
    }
    
    init();
  }, [initializeApp]);

  // Loading state
  if (loading) {
    return (
      <div className="app-loading">
        <div className="loading-spinner" />
        <h2>Initializing Ozone Studio...</h2>
        <p>Connecting to backend services</p>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div className="app-error">
        <h2>Connection Error</h2>
        <p>{error}</p>
        <button onClick={() => window.location.reload()}>
          Retry
        </button>
      </div>
    );
  }

  return (
    <div className="app">
      {/* Connection Bar - Always visible at top */}
      <ConnectionBar />
      
      {/* Main content area */}
      <div className="app-content">
        {/* Meta Portion - Always accessible */}
        <MetaPortion width={metaPortionWidth} />
        
        {/* Theme Area - Pipeline-driven content */}
        <ThemeArea theme={currentTheme} />
      </div>
    </div>
  );
}

export default App;
