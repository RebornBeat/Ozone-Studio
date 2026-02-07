/**
 * Ozone Studio - State Management
 * 
 * Uses Zustand for simple, effective state management.
 * The store does NOT replace pipeline logic - it only tracks UI state
 * and coordinates with pipelines via the ozone API.
 * 
 * KEY PRINCIPLE: No mock data. All values start at 0/empty and are
 * populated from the actual backend when connected.
 */

import { create } from 'zustand';
import { SystemStats } from '../App';

// ============================================================================
// Types
// ============================================================================

interface Task {
  id: number;
  pipelineId: number;
  status: 'queued' | 'running' | 'completed' | 'failed';
  progress: number;
  createdAt: number;
}

interface UIState {
  // Connection state
  isConnected: boolean;
  isAuthenticated: boolean;
  sessionToken: Uint8Array | null;
  
  // System stats (from backend - NO mock data)
  systemStats: SystemStats;
  
  // UI configuration
  currentTheme: string;
  metaPortionWidth: number;
  
  // Features
  consciousnessEnabled: boolean;
  p2pEnabled: boolean;
  
  // Active tasks
  activeTasks: Task[];
  
  // Prompt state (Meta Portion)
  promptInput: string;
  promptHistory: string[];
  
  // Navigation
  activeTab: 'workspace' | 'library' | 'settings';
  
  // Model selection (from config)
  selectedModel: string;
  availableModels: Array<{ name: string; model_type: string; identifier: string }>;
}

interface UIActions {
  // Initialization
  initializeApp: (config: any) => Promise<void>;
  
  // Connection
  setConnectionStatus: (connected: boolean) => void;
  setSystemStats: (stats: Partial<SystemStats>) => void;
  
  // Authentication
  authenticate: () => Promise<void>;
  logout: () => void;
  
  // Pipeline execution
  executePipeline: (pipelineId: number, input: object) => Promise<number>;
  
  // Orchestration (full 14-stage flow)
  orchestratePrompt: (prompt: string, projectId?: number, workspaceId?: number) => Promise<any>;
  
  // Task management
  updateTaskStatus: (taskId: number, status: Task['status'], progress?: number) => void;
  
  // UI state
  setPromptInput: (input: string) => void;
  submitPrompt: () => Promise<void>;
  setActiveTab: (tab: UIState['activeTab']) => void;
  setTheme: (theme: string) => void;
  setSelectedModel: (model: string) => void;
  setConsciousnessEnabled: (enabled: boolean) => void;
}

// Default system stats - ALL ZEROS (no mock data!)
const defaultSystemStats: SystemStats = {
  backendConnected: false,
  p2pEnabled: false,
  peerCount: 0,
  totalContributions: 0,
  myContributions: 0,
  methodologiesShared: 0,
  blueprintsShared: 0,
  findingsShared: 0,
  zseiContainers: 0,
  zseiDepth: 0,
  consciousnessEnabled: false,
  consciousnessState: undefined,
  iLoopStatus: undefined,
  uptime: 0,
  memoryUsage: 0,
  activeTaskCount: 0,
};

// ============================================================================
// Store
// ============================================================================

export const useOzoneStore = create<UIState & UIActions>((set, get) => ({
  // Initial state - NO mock data
  isConnected: false,
  isAuthenticated: false,
  sessionToken: null,
  systemStats: defaultSystemStats,
  currentTheme: 'home_dashboard',
  metaPortionWidth: 20,
  consciousnessEnabled: false,
  p2pEnabled: false,
  activeTasks: [],
  promptInput: '',
  promptHistory: [],
  activeTab: 'workspace',
  selectedModel: 'claude-sonnet-4-20250514',
  availableModels: [],
  
  // Actions
  initializeApp: async (config: any) => {
    // Extract configuration
    const uiConfig = config.ui || {};
    const modelConfig = config.models || {};
    const featuresConfig = config.features || {};
    
    set({
      currentTheme: uiConfig.theme || 'home_dashboard',
      metaPortionWidth: uiConfig.meta_portion_width_percent || 20,
      selectedModel: modelConfig.api_model || modelConfig.local_model_path || 'claude-sonnet-4-20250514',
      availableModels: modelConfig.available_models || [],
      consciousnessEnabled: featuresConfig.consciousness_enabled || false,
      p2pEnabled: featuresConfig.p2p_enabled || false,
    });
    
    // Load theme via ThemeLoader pipeline (id: 2) - only if connected
    if (window.ozone) {
      const { executePipeline } = get();
      try {
        await executePipeline(2, { theme: uiConfig.theme || 'home_dashboard' });
      } catch (err) {
        console.warn('Failed to load theme:', err);
      }
    }
  },
  
  setConnectionStatus: (connected: boolean) => {
    set({ 
      isConnected: connected,
      systemStats: {
        ...get().systemStats,
        backendConnected: connected,
      },
    });
  },
  
  setSystemStats: (stats: Partial<SystemStats>) => {
    set({
      systemStats: {
        ...get().systemStats,
        ...stats,
      },
    });
  },
  
  authenticate: async () => {
    if (!window.ozone) {
      console.warn('Backend not connected - cannot authenticate');
      return;
    }
    
    try {
      // Generate device-based authentication keys
      // In production, these would be derived from secure device storage
      const deviceId = await window.ozone.system?.getDeviceId?.() || 'default-device';
      const deviceIdBytes = new TextEncoder().encode(deviceId);
      
      // Create a deterministic public key from device ID (simplified for demo)
      // In production, use proper key derivation (e.g., HKDF)
      const publicKey = new Uint8Array(32);
      for (let i = 0; i < 32; i++) {
        publicKey[i] = deviceIdBytes[i % deviceIdBytes.length] ^ (i * 7);
      }
      
      // Create signature (in production, sign with actual private key)
      const signature = new Uint8Array(64);
      const timestamp = Date.now();
      const timestampBytes = new TextEncoder().encode(timestamp.toString());
      for (let i = 0; i < 64; i++) {
        signature[i] = (publicKey[i % 32] ^ timestampBytes[i % timestampBytes.length]) & 0xFF;
      }
      
      const result = await window.ozone.auth.authenticate(publicKey, signature);
      
      if (result.success && result.sessionToken) {
        set({
          isAuthenticated: true,
          sessionToken: result.sessionToken,
        });
      }
    } catch (err) {
      console.error('Authentication failed:', err);
    }
  },
  
  logout: () => {
    set({
      isAuthenticated: false,
      sessionToken: null,
    });
  },
  
  executePipeline: async (pipelineId: number, input: object) => {
    if (!window.ozone) {
      console.warn('Backend not connected - cannot execute pipeline');
      throw new Error('Backend not connected');
    }
    
    const result = await window.ozone.pipeline.execute(pipelineId, input);
    
    if (result.success) {
      const task: Task = {
        id: result.taskId,
        pipelineId,
        status: 'running',
        progress: 0,
        createdAt: Date.now(),
      };
      
      set((state) => ({
        activeTasks: [...state.activeTasks, task],
        systemStats: {
          ...state.systemStats,
          activeTaskCount: state.systemStats.activeTaskCount + 1,
        },
      }));
      
      return result.taskId;
    } else {
      throw new Error(result.error || 'Pipeline execution failed');
    }
  },
  
  updateTaskStatus: (taskId: number, status: Task['status'], progress = 0) => {
    set((state) => {
      const newTasks = state.activeTasks.map((task) =>
        task.id === taskId ? { ...task, status, progress } : task
      );
      
      // Update active task count
      const runningCount = newTasks.filter(t => t.status === 'running' || t.status === 'queued').length;
      
      return {
        activeTasks: newTasks,
        systemStats: {
          ...state.systemStats,
          activeTaskCount: runningCount,
        },
      };
    });
  },
  
  setPromptInput: (input: string) => {
    set({ promptInput: input });
  },
  
  submitPrompt: async () => {
    const { promptInput, orchestratePrompt, isConnected } = get();
    
    if (!promptInput.trim()) return;
    
    // Add to history
    set((state) => ({
      promptHistory: [...state.promptHistory, promptInput],
      promptInput: '',
    }));
    
    if (!isConnected) {
      console.warn('Backend not connected - prompt not executed');
      return;
    }
    
    // Use the orchestrator for full 14-stage flow
    try {
      await orchestratePrompt(promptInput);
    } catch (err) {
      console.error('Prompt orchestration failed:', err);
    }
  },
  
  /**
   * Orchestrate a prompt through the full 14-stage flow
   * This is the proper way to execute prompts - NOT directly calling pipeline 9
   */
  orchestratePrompt: async (prompt: string, projectId?: number, workspaceId?: number) => {
    const { consciousnessEnabled, selectedModel } = get();
    
    if (!window.ozone) {
      throw new Error('Backend not connected');
    }
    
    // Check if orchestrator is available
    if (window.ozone.orchestrate) {
      // Get user_id from auth session if available, fallback to device ID
      const userId = window.ozone.auth?.getCurrentUserId?.() ?? 1;
      const deviceId = window.ozone.system?.getDeviceId?.() ?? 1;
      
      // Use the orchestrator endpoint
      const result = await window.ozone.orchestrate({
        prompt,
        project_id: projectId,
        workspace_id: workspaceId,
        user_id: userId,
        device_id: deviceId,
        consciousness_enabled: consciousnessEnabled,
        token_budget: 100000,
        model_config: {
          model_identifier: selectedModel,
        },
      });
      
      if (!result.success) {
        throw new Error(result.error || 'Orchestration failed');
      }
      
      // Update system stats with stage info
      set((state) => ({
        systemStats: {
          ...state.systemStats,
          activeTaskCount: state.systemStats.activeTaskCount + 1,
        },
      }));
      
      return result;
    } else {
      // Fallback to direct pipeline execution if orchestrator not available
      console.warn('Orchestrator not available, falling back to direct pipeline execution');
      const { executePipeline } = get();
      
      // First aggregate context if we have a project
      let aggregatedContext = '';
      if (projectId) {
        try {
          const contextResult = await window.ozone.pipeline.execute(21, {
            action: 'ForProject',
            project_id: projectId,
            token_budget: 50000,
          });
          aggregatedContext = contextResult?.context?.context_text || '';
        } catch (err) {
          console.warn('Context aggregation failed:', err);
        }
      }
      
      // Execute prompt pipeline with context
      await executePipeline(9, {
        prompt,
        aggregated_context: aggregatedContext,
        model: selectedModel,
      });
    }
  },
  
  setActiveTab: (tab: UIState['activeTab']) => {
    set({ activeTab: tab });
    
    if (!window.ozone) return;
    
    // Execute corresponding tab pipeline
    const { executePipeline } = get();
    const tabPipelineMap: Record<UIState['activeTab'], number> = {
      workspace: 6,
      library: 7,
      settings: 8,
    };
    
    executePipeline(tabPipelineMap[tab], {}).catch(console.warn);
  },
  
  setTheme: (theme: string) => {
    set({ currentTheme: theme });
    
    if (!window.ozone) return;
    
    const { executePipeline } = get();
    executePipeline(2, { theme }).catch(console.warn);
  },
  
  setSelectedModel: (model: string) => {
    set({ selectedModel: model });
    
    if (window.ozone) {
      window.ozone.config.set({ models: { api_model: model } }).catch(console.warn);
    }
  },
  
  setConsciousnessEnabled: (enabled: boolean) => {
    set({ 
      consciousnessEnabled: enabled,
      systemStats: {
        ...get().systemStats,
        consciousnessEnabled: enabled,
      },
    });
    
    if (window.ozone) {
      window.ozone.config.set({ features: { consciousness_enabled: enabled } }).catch(console.warn);
    }
  },
}));
