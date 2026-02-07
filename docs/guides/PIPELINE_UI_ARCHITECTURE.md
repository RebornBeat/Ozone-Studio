# Pipeline UI Architecture v0.4.0

## Overview

**ALL pipeline UIs are FULLY DYNAMIC** - loaded as JS modules at runtime.
NO REBUILD NEEDED when adding new pipeline UIs!

## Quick Start: Adding a Pipeline UI

1. Create the UI file:
   ```
   pipelines/core/{pipeline_name}/ui/component.js
   ```

2. Export a render function:
   ```javascript
   module.exports = {
     meta: { title: 'My Pipeline', icon: '🔧' },
     
     render: function(container, props, React, ReactDOM) {
       // Your UI code here
       const root = ReactDOM.createRoot(container);
       root.render(React.createElement('div', null, 'Hello!'));
       
       // Return cleanup function
       return () => root.unmount();
     }
   };
   ```

3. Done! ThemeLoader serves the JS, frontend loads and executes it.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     ThemeArea                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              HomeDashboard (Shell)                   │   │
│  │  ┌──────┬──────┬──────┬──────┬──────┐              │   │
│  │  │Tab 1 │Tab 2 │Tab 3 │Tab 4 │Tab N │ ← ALL DYNAMIC │   │
│  │  └──────┴──────┴──────┴──────┴──────┘              │   │
│  │                                                       │   │
│  │  ┌─────────────────────────────────────────────┐    │   │
│  │  │   DynamicPipelineUI                          │    │   │
│  │  │   (loads & executes JS module at runtime)    │    │   │
│  │  └─────────────────────────────────────────────┘    │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                   ThemeLoader (Pipeline #2)                  │
│  GetPipelineUI { pipeline_id } → returns JS code             │
│  HasPipelineUI { pipeline_id } → returns boolean             │
│  ListPipelineUIs → returns all pipelines with UIs            │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│              Pipeline Registry (BuiltinPipeline)             │
│  Defined in: src/types/pipeline.rs                           │
│  54 pipelines total (1-38 core, 39-54 consciousness)        │
└─────────────────────────────────────────────────────────────┘
```

## Render Function Props

Your render function receives:

| Prop | Type | Description |
|------|------|-------------|
| `container` | HTMLElement | DOM element to render into |
| `props.pipelineId` | number | This pipeline's ID |
| `props.executePipeline` | (id, input) => Promise | Call any pipeline |
| `props.getSharedState` | () => object | Get shared state across tabs |
| `props.setSharedState` | (updates) => void | Update shared state |
| `props.subscribeToState` | (listener) => unsubscribe | Listen to state changes |
| `props.onClose` | () => void | Close this tab (if closeable) |
| `props.initialData` | any | Data passed when tab was injected |
| `React` | module | React library |
| `ReactDOM` | module | ReactDOM library |

## Example Components

### Simple Form

```javascript
module.exports = {
  meta: { title: 'Link URL', icon: '🔗' },
  
  render: function(container, props, React, ReactDOM) {
    const { useState } = React;
    
    function URLLinkUI() {
      const [url, setUrl] = useState('');
      const [loading, setLoading] = useState(false);
      
      const handleSubmit = async (e) => {
        e.preventDefault();
        setLoading(true);
        try {
          await props.executePipeline(31, { action: 'Link', url });
          props.onClose?.();
        } finally {
          setLoading(false);
        }
      };
      
      return React.createElement('form', { onSubmit: handleSubmit },
        React.createElement('input', {
          value: url,
          onChange: (e) => setUrl(e.target.value),
          placeholder: 'https://...',
        }),
        React.createElement('button', { type: 'submit', disabled: loading },
          loading ? 'Linking...' : 'Link URL'
        )
      );
    }
    
    const root = ReactDOM.createRoot(container);
    root.render(React.createElement(URLLinkUI));
    return () => root.unmount();
  }
};
```

### Using Shared State

```javascript
module.exports = {
  render: function(container, props, React, ReactDOM) {
    const { useState, useEffect } = React;
    
    function SharedStateDemo() {
      const [state, setState] = useState(props.getSharedState());
      
      useEffect(() => {
        // Subscribe to state changes from other tabs
        return props.subscribeToState(setState);
      }, []);
      
      const updateShared = () => {
        props.setSharedState({ lastUpdated: Date.now() });
      };
      
      return React.createElement('div', null,
        React.createElement('pre', null, JSON.stringify(state, null, 2)),
        React.createElement('button', { onClick: updateShared }, 'Update Shared State')
      );
    }
    
    const root = ReactDOM.createRoot(container);
    root.render(React.createElement(SharedStateDemo));
    return () => root.unmount();
  }
};
```

## Core Tabs

Core tabs are also dynamic! They use pipeline IDs:

| Tab | Pipeline ID | Pipeline Name |
|-----|-------------|---------------|
| Workspace | 6 | WorkspaceTabPipeline |
| Tasks | 5 | TaskManagerPipeline |
| Library | 7 | LibraryTabPipeline |
| Settings | 8 | SettingsTabPipeline |

Note: TaskViewer (#36) is deprecated - task viewing is now part of TaskManager (#5).

## Tab Injection

Tabs are injected via ThemeArea's exposed API:

```javascript
// Inject a tab programmatically
window.__ozoneThemeArea.injectTab(pipelineId, {
  id: 'custom-id',       // Optional
  label: 'My Tab',       // Optional
  icon: '🔧',           // Optional
  makeActive: true,      // Optional
  initialData: { ... },  // Optional
});

// Remove a tab
window.__ozoneThemeArea.uninjectTab('custom-id');
```

## TaskManager Integration

TaskManager (#5) has hooks to inject pipeline UIs during task execution:

- `onStepActive(pipelineId, stepData)` - Called when a step becomes active
- `onStepComplete(pipelineId)` - Called when a step completes

These hooks automatically inject/uninject tabs for pipelines that have UIs.

## File Structure

```
pipelines/core/
  auth/
    main.rs
  workspace_tab/
    main.rs
    ui/
      component.js    ← Dynamic UI
  task_manager/
    main.rs
    ui/
      component.js    ← Dynamic UI
  file_link/
    main.rs
    ui/
      component.js    ← Dynamic UI
  ...

ui/src/
  pipeline-ui/
    index.ts              ← Exports
    DynamicPipelineUI.tsx ← Runtime loader
    PipelineUILoader.ts   ← Module loading
```

## Pipeline Registry

Uses `BuiltinPipeline` enum from `src/types/pipeline.rs`:
- 1-38: Core system pipelines
- 39-54: Consciousness pipelines

The registry is the source of truth - no hardcoding pipeline IDs elsewhere.

## Best Practices

1. **Always return a cleanup function** - Unmount React, remove listeners
2. **Use shared state for cross-tab communication** - Don't rely on global variables
3. **Keep UIs focused** - One purpose per pipeline UI
4. **Handle loading and error states** - Give users feedback
5. **Use props.executePipeline** - Don't call window.ozone directly

## Troubleshooting

### UI Not Loading
- Check file path: `pipelines/core/{name}/ui/component.js`
- Verify export: `module.exports = { render: ... }`
- Check browser console for JS errors

### Module Execution Errors
- Ensure the JS is valid and doesn't throw on load
- Check that all referenced variables exist
- Use try/catch in your render function

