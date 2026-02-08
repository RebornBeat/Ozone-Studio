# OZONE STUDIO v0.4.0 - Pipeline UI Architecture

## Overview

OZONE Studio uses a **dynamic pipeline UI loading system** where each pipeline can have its own UI component that is loaded at runtime without rebuilding the frontend. This document explains the architecture and how to add new pipeline UIs.

## Directory Structure

```
Ozone-Studio/
├── ui/                                    # Electron/React frontend
│   ├── src/
│   │   ├── App.tsx                       # Main app entry
│   │   ├── themes/
│   │   │   └── HomeDashboard.tsx         # Theme shell (tab bar + content area)
│   │   ├── services/
│   │   │   └── PipelineUILoader.tsx      # Dynamic component loader
│   │   └── pipeline-ui/                  # ⚠️ DEPRECATED - DO NOT USE
│   │       └── *.tsx                     # These files should be DELETED
│   └── ...
│
├── pipelines/
│   ├── core/                             # Core system pipelines (1-38)
│   │   ├── workspace_tab/                # Pipeline #6
│   │   │   ├── main.rs                   # Rust backend logic
│   │   │   └── ui/
│   │   │       └── component.js          # ✅ CORRECT location for UI
│   │   ├── task_manager/                 # Pipeline #5
│   │   │   ├── main.rs
│   │   │   └── ui/
│   │   │       └── component.js
│   │   ├── library_tab/                  # Pipeline #7
│   │   │   ├── main.rs
│   │   │   └── ui/
│   │   │       └── component.js
│   │   ├── settings_tab/                 # Pipeline #8
│   │   │   ├── main.rs
│   │   │   └── ui/
│   │   │       └── component.js
│   │   ├── file_link/                    # Pipeline #30 - NO TAB (called from WorkspaceTab)
│   │   │   └── main.rs
│   │   ├── url_link/                     # Pipeline #31 - NO TAB (called from WorkspaceTab)
│   │   │   └── main.rs
│   │   └── ...
│   │
│   └── consciousness/                    # Consciousness pipelines (39-54)
│       ├── consciousness_decision_gate/  # Pipeline #39
│       │   ├── main.rs
│       │   └── ui/
│       │       └── component.js          # Optional UI
│       └── ...
│
└── src/
    └── pipeline/
        └── registry.rs                   # SOURCE OF TRUTH for pipeline IDs
```

## Core Concepts

### 1. Themes vs Pipelines

**Theme** (e.g., `home_dashboard`):
- Provides the shell/container (tab bar + content area)
- Defines which tabs to show by default
- Managed by `ThemeLoaderPipeline` (#2)

**Pipeline UI**:
- The actual content for each tab
- Lives in `pipelines/{category}/{name}/ui/component.js`
- Loaded dynamically by `PipelineUILoader`

### 2. Tab Types

**Core Tabs** (always visible, never uninjected):
- Workspace (#6)
- Tasks (#5 - TaskManager)
- Library (#7)
- Settings (#8)

**Pipeline Tabs** (injected during tasks, removed on completion):
- Appear after Settings
- Can be set active on injection
- Closeable by user

### 3. Component Loading Flow

```
1. App.tsx boots
2. Calls ThemeLoader.Load("home_dashboard")
3. Theme returns required_pipelines: [6, 5, 7, 8]
4. For each pipeline:
   a. PipelineUILoader calls GetPipelineUIComponent
   b. Backend reads pipelines/core/{name}/ui/component.js
   c. Returns JS content as string
   d. Frontend evaluates and renders component
5. Components are injected as tabs into HomeDashboard
```

## component.js Format

Each pipeline UI is a CommonJS module that exports:

```javascript
module.exports = {
  meta: {
    title: 'Tab Title',
    icon: '📁',           // Emoji or icon name
    version: '0.4.0',
  },
  
  // Main render function
  render: function(container, props, React, ReactDOM) {
    const { useState, useEffect } = React;
    const { executePipeline, getSharedState, setSharedState } = props;
    
    function MyComponent() {
      // Your React component logic here
      return React.createElement('div', null, 'Hello');
    }
    
    const root = ReactDOM.createRoot(container);
    root.render(React.createElement(MyComponent));
    
    // Return cleanup function
    return () => root.unmount();
  },
  
  // Optional lifecycle hooks
  onActivate: function() {
    console.log('Tab activated');
  },
  
  onDeactivate: function() {
    console.log('Tab deactivated');
  },
};
```

### Props Available

| Prop | Type | Description |
|------|------|-------------|
| `executePipeline(id, input)` | Function | Call any pipeline |
| `getSharedState()` | Function | Get shared state between tabs |
| `setSharedState(updates)` | Function | Update shared state |
| `pipelineId` | Number | This pipeline's ID |
| `isActive` | Boolean | Whether tab is currently active |

### Why React.createElement Instead of JSX?

The component.js files use `React.createElement()` instead of JSX because:
1. No build step required - files are loaded at runtime
2. No need to transpile JSX syntax
3. Components work immediately without Babel/TypeScript

## Core Tab Components

### WorkspaceTab (Pipeline #6)

**Location:** `pipelines/core/workspace_tab/ui/component.js`

**Features:**
- Workspace management (create, list, select)
- Project management within workspaces
- File linking via FileLink (#30)
- URL linking via URLLink (#31)
- Project-scoped chat with context aggregation

**Key Functions:**
```javascript
// List workspaces
executePipeline(6, { action: 'ListWorkspaces' })

// Create workspace
executePipeline(6, { action: 'CreateWorkspace', name: '...', description: '...' })

// List projects in workspace
executePipeline(6, { action: 'ListProjects', workspace_id: 123 })

// Create project
executePipeline(6, { action: 'CreateProject', workspace_id: 123, name: '...', description: '...' })

// Link file (calls FileLink pipeline)
executePipeline(30, { action: 'Link', project_id: 456, file_path: '/path/to/file' })

// Link URL (calls URLLink pipeline)
executePipeline(31, { action: 'Link', project_id: 456, url: 'https://...' })
```

### TaskManager (Pipeline #5)

**Location:** `pipelines/core/task_manager/ui/component.js`

**Features:**
- View all tasks (active, completed, failed)
- Task details and logs
- Cancel/retry tasks
- Real-time status updates

**Note:** TaskViewer (#36) is DEPRECATED and merged into TaskManager.

### LibraryTab (Pipeline #7)

**Location:** `pipelines/core/library_tab/ui/component.js`

**Features:**
- Browse methodologies
- Browse blueprints
- Search and filter
- View details

### SettingsTab (Pipeline #8)

**Location:** `pipelines/core/settings_tab/ui/component.js`

**Features:**
- Model configuration (API/Local)
- Voice input settings
- Consciousness system toggle
- P2P network settings
- Data export/import
- Clear all data

## Adding a New Pipeline UI

### Step 1: Create the component.js

```bash
mkdir -p pipelines/core/my_pipeline/ui
```

Create `pipelines/core/my_pipeline/ui/component.js`:

```javascript
module.exports = {
  meta: {
    title: 'My Pipeline',
    icon: '🔧',
    version: '0.4.0',
  },
  
  render: function(container, props, React, ReactDOM) {
    const { useState, useEffect } = React;
    const { executePipeline } = props;
    
    function MyPipelineUI() {
      const [data, setData] = useState(null);
      const [loading, setLoading] = useState(true);
      
      useEffect(() => {
        loadData();
      }, []);
      
      const loadData = async () => {
        setLoading(true);
        try {
          const result = await executePipeline(YOUR_PIPELINE_ID, { action: 'GetData' });
          setData(result);
        } catch (e) {
          console.error('Failed to load:', e);
        }
        setLoading(false);
      };
      
      if (loading) {
        return React.createElement('div', { className: 'loading' }, 'Loading...');
      }
      
      return React.createElement('div', { className: 'my-pipeline' },
        React.createElement('h2', null, 'My Pipeline'),
        React.createElement('pre', null, JSON.stringify(data, null, 2))
      );
    }
    
    const root = ReactDOM.createRoot(container);
    root.render(React.createElement(MyPipelineUI));
    return () => root.unmount();
  },
};
```

### Step 2: Register in registry.rs (if new pipeline)

Add to `src/pipeline/registry.rs`:

```rust
XX => ("core", "my_pipeline"),  // Replace XX with your pipeline ID
```

### Step 3: Update ThemeLoader (if it should be a tab)

To make it appear as a core tab, add to `ThemeLoaderPipeline`:

```rust
required_pipelines: vec![5, 6, 7, 8, XX],  // Add your ID
tabs: vec![
    // ... existing tabs ...
    ThemeTab { id: "my_pipeline".into(), name: "My Pipeline".into(), icon: Some("🔧".into()), pipeline_id: Some(XX) },
],
```

### Step 4: Test

No rebuild needed! Just restart the backend and refresh the UI.

## Pipelines WITHOUT Tabs

Some pipelines are called from other UIs but don't have their own tab:

| Pipeline | ID | Called From |
|----------|-----|-------------|
| FileLink | 30 | WorkspaceTab |
| URLLink | 31 | WorkspaceTab |
| PackageLink | 32 | WorkspaceTab |
| ContextAggregation | 21 | Orchestrator, WorkspaceTab chat |
| Prompt | 9 | Orchestrator, WorkspaceTab chat |

These pipelines don't need a `ui/` folder.

## Tab Injection at Runtime

Tabs can be injected/uninjected during task execution:

```javascript
// From backend or frontend
window.ozone.pipeline.execute(2, {
  action: 'InjectTab',
  pipeline_id: 25,        // BrowserNavigation
  label: 'Browser',
  icon: '🌐',
  make_active: true
});

// Later, when task completes
window.ozone.pipeline.execute(2, {
  action: 'UninjectTab',
  tab_id: 'browser_25'
});
```

## CSS Styling

Each component can use these predefined CSS classes:

```css
/* Containers */
.workspace-tab, .settings-panel, .task-panel { ... }
.panel-toolbar { ... }
.panel-loading { ... }
.panel-error { ... }
.panel-empty-centered { ... }

/* Cards and Items */
.item-card, .settings-card { ... }
.card-header, .card-body { ... }

/* Forms */
.setting-group { ... }
.radio-group, .checkbox-label { ... }
.file-input-group { ... }

/* Modals */
.modal-overlay { ... }
.modal { ... }
.modal-actions { ... }

/* Buttons */
.btn-primary, .btn-secondary, .btn-danger { ... }
.toolbar-btn { ... }
```

## Deprecated Files (DELETE THESE)

The following files in `ui/src/pipeline-ui/` are DEPRECATED and should be deleted:

- `WorkspaceTab.tsx` - ❌ DELETE (replaced by `pipelines/core/workspace_tab/ui/component.js`)
- `SettingsTab.tsx` - ❌ DELETE (replaced by `pipelines/core/settings_tab/ui/component.js`)
- `TasksTab.tsx` - ❌ DELETE (replaced by `pipelines/core/task_manager/ui/component.js`)
- `LibraryTab.tsx` - ❌ DELETE (replaced by `pipelines/core/library_tab/ui/component.js`)

The correct UI files are in each pipeline's `ui/` folder, NOT in the frontend source.

## Summary

| What | Where |
|------|-------|
| Tab UI components | `pipelines/{category}/{name}/ui/component.js` |
| Theme shells | `ui/src/themes/` |
| Pipeline registry | `src/pipeline/registry.rs` |
| Dynamic loader | `ui/src/services/PipelineUILoader.tsx` |
| Deprecated (DELETE) | `ui/src/pipeline-ui/*.tsx` |

## Quick Reference: Pipeline IDs

### Core Tabs
- **5** - TaskManager (Tasks tab)
- **6** - WorkspaceTab (Workspace tab)
- **7** - LibraryTab (Library tab)
- **8** - SettingsTab (Settings tab)

### Called by WorkspaceTab
- **9** - Prompt (chat)
- **21** - ContextAggregation (chat context)
- **30** - FileLink
- **31** - URLLink
- **32** - PackageLink

### System Pipelines
- **1** - Auth
- **2** - ThemeLoader
- **3** - ZSEIQuery
- **4** - ZSEIWrite

### Consciousness (39-54)
- **39** - ConsciousnessDecisionGate
- **44** - ILoop
- **49** - EthicalSimulation
- etc.
