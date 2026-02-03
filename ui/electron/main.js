/**
 * Ozone Studio - Electron Main Process v0.4.0
 * 
 * BEHAVIOR:
 * 1. Check if backend is running
 * 2. If not, show countdown (5 seconds) then try to launch backend ONCE
 * 3. Continue checking for connection - UI stays on loading screen until connected
 * 4. Once connected, check if first launch and show setup wizard if needed
 * 
 * IMPORTANT: UI navigation does NOT create tasks. Only PromptPipeline creates tasks.
 */

const { app, BrowserWindow, ipcMain } = require('electron');
const path = require('path');
const http = require('http');
const { spawn } = require('child_process');
const fs = require('fs');

let mainWindow = null;
let isBackendConnected = false;
let backendProcess = null;
let hasAttemptedAutoLaunch = false;

// Configuration
const BACKEND_HOST = '127.0.0.1';
const BACKEND_PORT = 50051;
const DEV_SERVER = 'http://localhost:5173';
const AUTO_LAUNCH_DELAY_SECONDS = 5;

/**
 * Check if the backend is running via HTTP health check
 */
function checkBackendConnection() {
  return new Promise((resolve) => {
    const req = http.request({
      hostname: BACKEND_HOST,
      port: BACKEND_PORT,
      path: '/health',
      method: 'GET',
      timeout: 2000,
    }, (res) => {
      resolve(true);
    });
    
    req.on('error', () => resolve(false));
    req.on('timeout', () => {
      req.destroy();
      resolve(false);
    });
    
    req.end();
  });
}

/**
 * Try to launch the backend process (once only, silently)
 */
function tryLaunchBackend() {
  if (hasAttemptedAutoLaunch) return false;
  hasAttemptedAutoLaunch = true;
  
  const isProduction = app.isPackaged;
  const backendPath = isProduction
    ? path.join(process.resourcesPath, 'ozone-studio')
    : path.join(__dirname, '..', '..', 'target', 'release', 'ozone-studio');
  
  console.log('Attempting to auto-launch backend:', backendPath);
  
  // Check if file exists
  if (!fs.existsSync(backendPath)) {
    console.log('Backend binary not found at:', backendPath);
    if (mainWindow) {
      mainWindow.webContents.send('backend-launch-status', {
        success: false,
        error: 'Backend not found. Start manually: ./ozone-studio',
        path: backendPath
      });
    }
    return false;
  }
  
  try {
    backendProcess = spawn(backendPath, [], {
      detached: false,
      stdio: ['ignore', 'pipe', 'pipe'],
      env: {
        ...process.env,
        OZONE_CONFIG: isProduction 
          ? path.join(app.getPath('userData'), 'config.toml')
          : path.join(__dirname, '..', '..', 'target', 'release', 'config.toml'),
      },
    });
    
    backendProcess.stdout.on('data', (data) => {
      console.log('[Backend]', data.toString().trim());
    });
    
    backendProcess.stderr.on('data', (data) => {
      console.error('[Backend Error]', data.toString().trim());
    });
    
    backendProcess.on('error', (err) => {
      console.error('Failed to start backend:', err.message);
      if (mainWindow) {
        mainWindow.webContents.send('backend-launch-status', {
          success: false,
          error: `Launch failed: ${err.message}`
        });
      }
    });
    
    backendProcess.on('close', (code) => {
      console.log('Backend process exited with code:', code);
      backendProcess = null;
    });
    
    if (mainWindow) {
      mainWindow.webContents.send('backend-launch-status', {
        success: true,
        message: 'Backend launched, awaiting connection...'
      });
    }
    
    return true;
  } catch (err) {
    console.error('Error spawning backend:', err.message);
    if (mainWindow) {
      mainWindow.webContents.send('backend-launch-status', {
        success: false,
        error: `Spawn error: ${err.message}`
      });
    }
    return false;
  }
}

/**
 * Monitor backend connection with countdown and auto-launch
 */
let countdownStarted = false;
let countdownValue = AUTO_LAUNCH_DELAY_SECONDS;

async function monitorBackendConnection() {
  const wasConnected = isBackendConnected;
  isBackendConnected = await checkBackendConnection();
  
  // Connection state changed
  if (isBackendConnected !== wasConnected && mainWindow) {
    mainWindow.webContents.send('connection-change', { connected: isBackendConnected });
    console.log('Backend connection:', isBackendConnected ? 'CONNECTED' : 'DISCONNECTED');
    
    if (isBackendConnected) {
      countdownStarted = false;
      countdownValue = AUTO_LAUNCH_DELAY_SECONDS;
    }
  }
  
  // Handle countdown and auto-launch when not connected
  if (!isBackendConnected) {
    if (!countdownStarted) {
      countdownStarted = true;
      countdownValue = AUTO_LAUNCH_DELAY_SECONDS;
    }
    
    // Send countdown to UI
    if (mainWindow) {
      mainWindow.webContents.send('connection-countdown', {
        secondsUntilRetry: Math.max(0, countdownValue),
        willAutoLaunch: !hasAttemptedAutoLaunch,
        hasAttemptedLaunch: hasAttemptedAutoLaunch
      });
    }
    
    // Decrement countdown
    if (countdownValue > 0) {
      countdownValue--;
    } else if (!hasAttemptedAutoLaunch) {
      // Time to try auto-launch (once only)
      tryLaunchBackend();
    }
  }
  
  // Check every second during countdown, every 3 seconds after
  const interval = (!isBackendConnected && !hasAttemptedAutoLaunch) ? 1000 : 3000;
  setTimeout(monitorBackendConnection, interval);
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
    title: 'Ozone Studio',
    show: false,
  });
  
  if (process.env.NODE_ENV === 'development') {
    mainWindow.loadURL(DEV_SERVER);
    mainWindow.webContents.openDevTools();
  } else {
    mainWindow.loadFile(path.join(__dirname, '..', 'dist', 'index.html'));
  }
  
  mainWindow.once('ready-to-show', () => {
    mainWindow.show();
  });
  
  mainWindow.on('closed', () => {
    mainWindow = null;
  });
}

// ============================================================================
// IPC Handlers - Communication with Rust Backend
// ============================================================================

async function backendRequest(method, path, body = null) {
  return new Promise((resolve, reject) => {
    const options = {
      hostname: BACKEND_HOST,
      port: BACKEND_PORT,
      path: path,
      method: method,
      headers: { 'Content-Type': 'application/json' },
      timeout: 10000,
    };
    
    const req = http.request(options, (res) => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => {
        try {
          resolve(JSON.parse(data));
        } catch (e) {
          resolve(data);
        }
      });
    });
    
    req.on('error', reject);
    req.on('timeout', () => {
      req.destroy();
      reject(new Error('Request timeout'));
    });
    
    if (body) req.write(JSON.stringify(body));
    req.end();
  });
}

function requireConnection() {
  if (!isBackendConnected) {
    throw new Error('Backend not connected');
  }
}

// Configuration
ipcMain.handle('config:get', async () => {
  requireConnection();
  const result = await backendRequest('POST', '/config/get', { session_token: '' });
  return {
    backend_connected: true,
    is_first_launch: !result.config?.setup_complete,
    ...result.config
  };
});

ipcMain.handle('config:set', async (event, updates) => {
  requireConnection();
  return await backendRequest('POST', '/config/set', { updates, session_token: '' });
});

// System
ipcMain.handle('system:getStats', async () => {
  requireConnection();
  const health = await backendRequest('GET', '/health');
  return {
    backendConnected: true,
    p2pEnabled: true,
    peerCount: 0,
    totalContributions: 0,
    myContributions: 0,
    methodologiesShared: 0,
    blueprintsShared: 0,
    findingsShared: 0,
    zseiContainers: 1,
    zseiDepth: 0,
    consciousnessEnabled: false,
    uptime: health.uptime_secs || 0,
    memoryUsage: 0,
    activeTaskCount: health.active_tasks || 0,
  };
});

ipcMain.handle('system:isFirstLaunch', async () => {
  try {
    requireConnection();
    const config = await backendRequest('POST', '/config/get', { session_token: '' });
    return !config.config?.setup_complete;
  } catch (err) {
    return true;
  }
});

ipcMain.handle('system:markSetupComplete', async () => {
  requireConnection();
  return await backendRequest('POST', '/config/set', { 
    updates: { setup_complete: true },
    session_token: '' 
  });
});

// Authentication
ipcMain.handle('auth:challenge', async (event, publicKey) => {
  requireConnection();
  return await backendRequest('POST', '/auth/challenge', { public_key: publicKey });
});

ipcMain.handle('auth:authenticate', async (event, { publicKey, signature }) => {
  requireConnection();
  return await backendRequest('POST', '/auth/authenticate', { 
    public_key: publicKey, 
    signature: signature 
  });
});

// Pipeline execution - ONLY this creates tasks (via PromptPipeline)
ipcMain.handle('pipeline:execute', async (event, { pipelineId, input, sessionToken }) => {
  requireConnection();
  return await backendRequest('POST', '/pipeline/execute', {
    pipeline_id: pipelineId,
    input: input,
    session_token: sessionToken || ''
  });
});

ipcMain.handle('pipeline:list', async () => {
  requireConnection();
  return {
    pipelines: [
      { id: 1, name: 'AuthPipeline' },
      { id: 2, name: 'ThemeLoaderPipeline' },
      { id: 3, name: 'ZSEIQueryPipeline' },
      { id: 9, name: 'PromptPipeline' },
      { id: 10, name: 'VoicePipeline' },
      { id: 11, name: 'SettingsPipeline' },
    ],
  };
});

// Task management - ONLY retrieves tasks, UI NEVER creates tasks directly
ipcMain.handle('task:list', async () => {
  requireConnection();
  return await backendRequest('POST', '/task/list', { session_token: '' });
});

ipcMain.handle('task:status', async (event, taskId) => {
  requireConnection();
  return await backendRequest('POST', '/task/get', { task_id: taskId, session_token: '' });
});

ipcMain.handle('task:cancel', async (event, taskId) => {
  requireConnection();
  return await backendRequest('POST', '/task/cancel', { task_id: taskId, session_token: '' });
});

// ZSEI queries
ipcMain.handle('zsei:query', async (event, query) => {
  requireConnection();
  return await backendRequest('POST', '/zsei/query', { query, session_token: '' });
});

// ============================================================================
// App Lifecycle
// ============================================================================

app.whenReady().then(async () => {
  console.log('╔═══════════════════════════════════════════════════════════════════╗');
  console.log('║                    OZONE STUDIO UI v0.4.0                         ║');
  console.log('╚═══════════════════════════════════════════════════════════════════╝');
  console.log('');
  
  isBackendConnected = await checkBackendConnection();
  console.log('Initial backend status:', isBackendConnected ? 'CONNECTED' : 'NOT RUNNING');
  
  if (!isBackendConnected) {
    console.log(`Will attempt auto-launch in ${AUTO_LAUNCH_DELAY_SECONDS} seconds...`);
  }
  
  monitorBackendConnection();
  createWindow();
  
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('before-quit', () => {
  if (backendProcess) {
    console.log('Stopping backend process...');
    backendProcess.kill();
  }
});
