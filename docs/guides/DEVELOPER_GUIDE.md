# OZONE STUDIO - Developer Guide v0.4.0

## Architecture Overview

OZONE STUDIO is built with a Rust backend and React/Electron frontend.

```
┌─────────────────────────────────────────────────────────────────┐
│                         ELECTRON UI                              │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────────────────┐ │
│  │ React App   │  │   Preload    │  │     Main Process        │ │
│  │ (Renderer)  │◄─┤   Bridge     │◄─┤  (IPC + HTTP Client)    │ │
│  └─────────────┘  └──────────────┘  └───────────┬─────────────┘ │
└─────────────────────────────────────────────────┼───────────────┘
                                                  │ HTTP
                                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│                       RUST BACKEND                               │
│  ┌──────────────┐  ┌─────────────┐  ┌───────────────────────┐   │
│  │  HTTP/WS     │  │   gRPC      │  │     OzoneRuntime      │   │
│  │  Server      │─►│   Module    │─►│  ┌─────────────────┐  │   │
│  │  (Axum)      │  │             │  │  │ Task Manager    │  │   │
│  └──────────────┘  └─────────────┘  │  │ Pipeline Exec   │  │   │
│                                      │  │ ZSEI Store      │  │   │
│                                      │  │ Auth Manager    │  │   │
│                                      │  │ Consciousness*  │  │   │
│                                      │  └─────────────────┘  │   │
│                                      └───────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Project Structure

```
Ozone-Studio/
├── src/                    # Rust backend
│   ├── main.rs            # Entry point
│   ├── lib.rs             # OzoneRuntime
│   ├── config/            # Configuration
│   ├── grpc/              # HTTP/WS service layer
│   ├── types/             # Type definitions
│   │   ├── mod.rs
│   │   ├── task.rs
│   │   ├── pipeline.rs
│   │   ├── consciousness.rs
│   │   └── ...
│   ├── pipeline/          # Pipeline execution
│   ├── task/              # Task management
│   ├── zsei/              # Zero-Shot Embedded Indexer
│   ├── consciousness/     # Consciousness module
│   ├── auth/              # Authentication
│   ├── p2p/               # Peer-to-peer networking
│   └── ...
├── ui/                     # Electron/React frontend
│   ├── electron/
│   │   ├── main.js        # Electron main process
│   │   └── preload.js     # IPC bridge
│   ├── src/
│   │   ├── App.tsx        # Main React app
│   │   ├── App.css        # Global styles
│   │   ├── components/    # React components
│   │   ├── themes/        # Theme implementations
│   │   └── services/      # Store & utilities
│   └── package.json
├── specs/                  # Formal specification
├── docs/                   # Documentation
├── Cargo.toml             # Rust dependencies
└── config.toml            # Runtime configuration
```

---

## Backend Development

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test
```

### Key Modules

#### OzoneRuntime (`src/lib.rs`)

The central runtime that coordinates all subsystems:

```rust
pub struct OzoneRuntime {
    pub config: OzoneConfig,
    pub task_manager: Arc<RwLock<TaskManager>>,
    pub pipeline_registry: Arc<RwLock<PipelineRegistry>>,
    pub zsei: Arc<RwLock<ZSEIStore>>,
    pub auth: Arc<RwLock<AuthManager>>,
    pub consciousness: Option<Arc<RwLock<ConsciousnessStore>>>,
    // ...
}
```

#### Configuration (`src/config/mod.rs`)

Configuration is loaded from `config.toml`:

```rust
pub struct OzoneConfig {
    pub general: GeneralConfig,    // data_dir, log_level, setup_complete
    pub zsei: ZSEIConfig,          // storage paths
    pub grpc: GrpcConfig,          // server address/port
    pub models: ModelConfig,       // LLM configuration
    pub consciousness: ConsciousnessConfig, // runtime toggle
    pub ui: UIConfig,              // theme settings
    // ...
}
```

#### gRPC Module (`src/grpc/mod.rs`)

HTTP endpoints for UI communication:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/config/get` | POST | Get configuration |
| `/config/set` | POST | Update configuration |
| `/auth/challenge` | POST | Request auth challenge |
| `/auth/authenticate` | POST | Authenticate user |
| `/pipeline/execute` | POST | Execute pipeline |
| `/task/list` | POST | List tasks |
| `/task/get` | POST | Get task status |
| `/zsei/query` | POST | Query ZSEI |
| `/ws` | GET | WebSocket connection |

#### Pipelines (`src/pipeline/mod.rs`)

Pipelines process different types of tasks:

```rust
// Core Pipelines (always loaded)
pub const BUILTIN_PIPELINES: &[BuiltinPipeline] = &[
    // Auth & Setup
    BuiltinPipeline { id: 1, name: "AuthPipeline", category: "core" },
    BuiltinPipeline { id: 2, name: "ThemeLoaderPipeline", category: "core" },
    // ... 38 core pipelines
    
    // Consciousness Pipelines (loaded when enabled)
    BuiltinPipeline { id: 39, name: "ConsciousnessDecisionGate", category: "consciousness" },
    // ... 16 consciousness pipelines
];
```

#### Consciousness (`src/consciousness/`)

Always compiled, runtime-controlled via `config.toml`:

```rust
// src/consciousness/mod.rs
pub type ConsciousnessSystem = store::ConsciousnessStore;

// Key types
pub struct NarrativeFragment { ... }
pub struct RelationshipContext { ... }
pub struct ConsciousnessDecisionGate { ... }
```

### Adding a New Pipeline

1. Define the pipeline in `src/pipeline/mod.rs`
2. Implement execution logic
3. Register in `BUILTIN_PIPELINES`
4. If UI needed, create UI component in `ui/src/themes/`

---

## Frontend Development

### Setup

```bash
cd ui
npm install
npm run dev      # Development server
npm run electron # Electron app
npm run build    # Production build
```

### Key Components

#### App.tsx

Main application with:
- Connection management
- Setup wizard
- Layout orchestration

#### MetaPortion.tsx

Left panel showing:
- Consciousness state (when enabled)
- Voice visualization
- Transcript
- Backup prompt input

#### HomeDashboard.tsx

Main theme with tabs:
- Workspace
- Tasks
- Library
- Settings

### State Management (Zustand)

```typescript
// ui/src/services/store.ts
interface UIState {
  isConnected: boolean;
  consciousnessEnabled: boolean;
  currentTheme: string;
  activeTasks: Task[];
  // ...
}

export const useOzoneStore = create<UIState & UIActions>((set, get) => ({
  // State and actions
}));
```

### IPC Communication

The preload bridge exposes:

```typescript
window.ozone = {
  config: { get, set },
  system: { getStats, isFirstLaunch, markSetupComplete },
  pipeline: { execute, list },
  task: { list, status, cancel },
  zsei: { query },
  auth: { challenge, authenticate },
  events: { 
    onConnectionChange, 
    onConnectionCountdown,
    onBackendLaunchStatus 
  },
};
```

### Adding UI Injections (Pipeline Tabs)

Pipelines can inject UI components:

1. Store UI components in pipeline's `ui/` folder
2. Register injection point in pipeline
3. ThemeLoader handles injection/uninjection
4. Tab appears dynamically in HomeDashboard

---

## Testing

### Backend Tests

```bash
cargo test                    # All tests
cargo test --lib              # Library tests only
cargo test pipeline::         # Pipeline tests
```

### Frontend Tests

```bash
cd ui
npm test                      # Run tests
npm run test:watch            # Watch mode
```

---

## Debugging

### Backend Logs

Set log level in `config.toml`:

```toml
[general]
log_level = "debug"  # trace, debug, info, warn, error
```

### Frontend DevTools

- In development mode, DevTools open automatically
- Access React DevTools for component inspection
- Check Network tab for HTTP requests

### Common Issues

1. **NarrativeFragment not found**: Use `crate::consciousness::NarrativeFragment`
2. **Consciousness feature flags**: Removed in v0.4.0, always compiled
3. **Tasks created by UI clicks**: Bug - only PromptPipeline creates tasks

---

## Code Style

### Rust

- Follow Rust 2021 edition conventions
- Use `#[derive(Debug, Clone, Serialize, Deserialize)]` for data types
- Document public APIs with `///`
- Group imports: std, external crates, internal modules

### TypeScript

- Use functional components with hooks
- Explicit typing for props and state
- No mock data - all values from backend

---

## Contributing

See [CONTRIBUTION_GUIDE.md](./CONTRIBUTION_GUIDE.md) for:
- Code contribution workflow
- PR requirements
- Review process

---

*OZONE STUDIO v0.4.0 - Developer Documentation*
