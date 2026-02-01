/**
 * Ozone Studio - State Management
 * 
 * Uses Zustand for simple, effective state management.
 * The store does NOT replace pipeline logic - it only tracks UI state
 * and coordinates with pipelines via the ozone API.
 */

import { create } from 'zustand';

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
  
  // UI configuration
  currentTheme: string;
  metaPortionWidth: number;
  
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
  
  // Authentication
  authenticate: () => Promise<void>;
  logout: () => void;
  
  // Pipeline execution
  executePipeline: (pipelineId: number, input: object) => Promise<number>;
  
  // Task management
  updateTaskStatus: (taskId: number, status: Task['status'], progress?: number) => void;
  
  // UI state
  setPromptInput: (input: string) => void;
  submitPrompt: () => Promise<void>;
  setActiveTab: (tab: UIState['activeTab']) => void;
  setTheme: (theme: string) => void;
  setSelectedModel: (model: string) => void;
}

// ============================================================================
// Store
// ============================================================================

export const useOzoneStore = create<UIState & UIActions>((set, get) => ({
  // Initial state
  isConnected: false,
  isAuthenticated: false,
  sessionToken: null,
  currentTheme: 'home_dashboard',
  metaPortionWidth: 20,
  activeTasks: [],
  promptInput: '',
  promptHistory: [],
  activeTab: 'workspace',
  selectedModel: 'claude-sonnet-4-20250514',
  availableModels: [],
  
  // Actions
  initializeApp: async (config: any) => {
    // Extract UI config
    const uiConfig = config.ui || {};
    const modelConfig = config.models || {};
    
    set({
      isConnected: true,
      currentTheme: uiConfig.theme || 'home_dashboard',
      metaPortionWidth: uiConfig.meta_portion_width_percent || 20,
      selectedModel: modelConfig.api_model || 'claude-sonnet-4-20250514',
      availableModels: modelConfig.available_models || [],
    });
    
    // Load theme via ThemeLoader pipeline (id: 2)
    const { executePipeline } = get();
    try {
      await executePipeline(2, { theme: uiConfig.theme || 'home_dashboard' });
    } catch (err) {
      console.warn('Failed to load theme:', err);
    }
  },
  
  authenticate: async () => {
    // This would use actual crypto keys in production
    // For now, we'll simulate authentication
    try {
      const publicKey = new Uint8Array(32); // Placeholder
      const result = await window.ozone.auth.authenticate(publicKey, new Uint8Array(64));
      
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
    // Execute pipeline via ozone API
    const result = await window.ozone.pipeline.execute(pipelineId, input);
    
    if (result.success) {
      // Add to active tasks
      const task: Task = {
        id: result.taskId,
        pipelineId,
        status: 'running',
        progress: 0,
        createdAt: Date.now(),
      };
      
      set((state) => ({
        activeTasks: [...state.activeTasks, task],
      }));
      
      return result.taskId;
    } else {
      throw new Error(result.error || 'Pipeline execution failed');
    }
  },
  
  updateTaskStatus: (taskId: number, status: Task['status'], progress = 0) => {
    set((state) => ({
      activeTasks: state.activeTasks.map((task) =>
        task.id === taskId ? { ...task, status, progress } : task
      ),
    }));
  },
  
  setPromptInput: (input: string) => {
    set({ promptInput: input });
  },
  
  submitPrompt: async () => {
    const { promptInput, executePipeline, selectedModel } = get();
    
    if (!promptInput.trim()) return;
    
    // Add to history
    set((state) => ({
      promptHistory: [...state.promptHistory, promptInput],
      promptInput: '',
    }));
    
    // Execute PromptPipeline (id: 9) with the input
    try {
      await executePipeline(9, {
        prompt: promptInput,
        model: selectedModel,
      });
    } catch (err) {
      console.error('Prompt execution failed:', err);
    }
  },
  
  setActiveTab: (tab: UIState['activeTab']) => {
    set({ activeTab: tab });
    
    // Execute corresponding tab pipeline
    const { executePipeline } = get();
    const tabPipelineMap: Record<UIState['activeTab'], number> = {
      workspace: 6,  // WorkspaceTabPipeline
      library: 7,    // LibraryTabPipeline
      settings: 8,   // SettingsTabPipeline
    };
    
    executePipeline(tabPipelineMap[tab], {}).catch(console.warn);
  },
  
  setTheme: (theme: string) => {
    set({ currentTheme: theme });
    
    // Execute ThemeLoader pipeline
    const { executePipeline } = get();
    executePipeline(2, { theme }).catch(console.warn);
  },
  
  setSelectedModel: (model: string) => {
    set({ selectedModel: model });
    
    // Update config
    window.ozone.config.set({ models: { api_model: model } }).catch(console.warn);
  },
}));
