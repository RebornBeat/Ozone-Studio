/**
 * Ozone Studio - Electron Main Process
 * 
 * This is the entry point for the desktop application.
 * It spawns the Rust backend and connects to it via gRPC.
 */

const { app, BrowserWindow, ipcMain } = require('electron');
const path = require('path');
const { spawn } = require('child_process');
const grpc = require('@grpc/grpc-js');
const protoLoader = require('@grpc/proto-loader');

let mainWindow = null;
let rustBackend = null;
let grpcClient = null;

// Configuration
const GRPC_ADDRESS = '127.0.0.1:50051';
const DEV_SERVER = 'http://localhost:5173';

/**
 * Start the Rust backend process
 */
function startRustBackend() {
  const isProduction = app.isPackaged;
  const backendPath = isProduction
    ? path.join(process.resourcesPath, 'ozone-studio')
    : path.join(__dirname, '..', '..', 'target', 'release', 'ozone-studio');
  
  console.log('Starting Rust backend:', backendPath);
  
  rustBackend = spawn(backendPath, [], {
    env: {
      ...process.env,
      OZONE_CONFIG: path.join(app.getPath('userData'), 'config.toml'),
    },
  });
  
  rustBackend.stdout.on('data', (data) => {
    console.log('[Rust]', data.toString());
  });
  
  rustBackend.stderr.on('data', (data) => {
    console.error('[Rust Error]', data.toString());
  });
  
  rustBackend.on('close', (code) => {
    console.log('Rust backend exited with code:', code);
    if (code !== 0 && mainWindow) {
      // Backend crashed - notify UI
      mainWindow.webContents.send('backend-error', { code });
    }
  });
}

/**
 * Initialize gRPC client to communicate with Rust backend
 */
function initGrpcClient() {
  // In production, load the proto file
  // For now, we'll use a simple approach
  console.log('Initializing gRPC client to', GRPC_ADDRESS);
  
  // The actual gRPC client will be initialized once proto definitions are ready
  // For now, we'll use IPC to simulate the connection
}

/**
 * Create the main application window
 */
function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1400,
    height: 900,
    minWidth: 800,
    minHeight: 600,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, 'preload.js'),
    },
    titleBarStyle: 'hiddenInset',
    show: false,
  });
  
  // Load the app
  if (process.env.NODE_ENV === 'development') {
    mainWindow.loadURL(DEV_SERVER);
    mainWindow.webContents.openDevTools();
  } else {
    mainWindow.loadFile(path.join(__dirname, '..', 'dist', 'index.html'));
  }
  
  // Show window when ready
  mainWindow.once('ready-to-show', () => {
    mainWindow.show();
  });
  
  mainWindow.on('closed', () => {
    mainWindow = null;
  });
}

// ============================================================================
// IPC Handlers - Bridge between UI and Rust backend
// ============================================================================

// Authentication
ipcMain.handle('auth:challenge', async (event, publicKey) => {
  // Forward to Rust backend via gRPC
  console.log('Auth challenge requested for:', publicKey);
  return { challenge: 'mock-challenge-' + Date.now() };
});

ipcMain.handle('auth:authenticate', async (event, { publicKey, signature }) => {
  console.log('Authentication requested');
  return { success: true, sessionToken: 'mock-session-' + Date.now() };
});

// Pipeline execution
ipcMain.handle('pipeline:execute', async (event, { pipelineId, input }) => {
  console.log('Pipeline execution requested:', pipelineId);
  return { success: true, taskId: Date.now(), output: {} };
});

ipcMain.handle('pipeline:list', async () => {
  // Return list of available pipelines
  return {
    pipelines: [
      { id: 1, name: 'AuthPipeline' },
      { id: 2, name: 'ThemeLoaderPipeline' },
      { id: 3, name: 'ZSEIQueryPipeline' },
      { id: 4, name: 'ZSEIWritePipeline' },
      { id: 5, name: 'TaskManagerPipeline' },
      { id: 6, name: 'WorkspaceTabPipeline' },
      { id: 7, name: 'LibraryTabPipeline' },
      { id: 8, name: 'SettingsTabPipeline' },
      { id: 9, name: 'PromptPipeline' },
      { id: 10, name: 'VoicePipeline' },
      // ... more pipelines
    ],
  };
});

// ZSEI queries
ipcMain.handle('zsei:query', async (event, query) => {
  console.log('ZSEI query:', query);
  return { result: [] };
});

// Task management
ipcMain.handle('task:status', async (event, taskId) => {
  console.log('Task status requested:', taskId);
  return { status: 'completed' };
});

ipcMain.handle('task:list', async () => {
  return { tasks: [] };
});

// Configuration
ipcMain.handle('config:get', async () => {
  return {
    models: {
      model_type: 'api',
      api_endpoint: 'https://api.anthropic.com/v1/messages',
      allow_user_selection: true,
      available_models: [
        { name: 'Claude Sonnet', model_type: 'api', identifier: 'claude-sonnet-4-20250514' },
      ],
    },
    ui: {
      theme: 'home_dashboard',
      meta_portion_width_percent: 20,
    },
  };
});

ipcMain.handle('config:set', async (event, updates) => {
  console.log('Config update:', updates);
  return { success: true };
});

// ============================================================================
// App Lifecycle
// ============================================================================

app.whenReady().then(() => {
  // Start Rust backend (in production)
  if (app.isPackaged) {
    startRustBackend();
  }
  
  // Initialize gRPC client
  initGrpcClient();
  
  // Create window
  createWindow();
  
  // macOS: Re-create window when dock icon is clicked
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

app.on('window-all-closed', () => {
  // Quit when all windows are closed (except on macOS)
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('before-quit', () => {
  // Stop Rust backend
  if (rustBackend) {
    rustBackend.kill();
  }
});
