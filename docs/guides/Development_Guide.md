# Ozone Studio — Development Guide

**Version:** 0.3  
**Last Updated:** 2025-01-18

---

## Overview

Ozone Studio is a **pipeline execution engine** with an Electron-based GUI. The core system provides:
- Authentication (pub/priv key)
- ZSEI (Zero-Shot Embedded Indexer) for knowledge storage
- Task management
- gRPC server for UI communication
- Pipeline registry and execution environment

**All actual functionality (LLM calls, code analysis, UI rendering, etc.) is provided by PIPELINES.**

---

## Prerequisites

### Required Software

| Software | Version | Purpose |
|----------|---------|---------|
| Rust | 1.75+ | Core engine |
| Node.js | 20+ | Electron UI |
| pnpm | 8+ | Package manager |
| PostgreSQL | 15+ | Metadata storage |
| protoc | 3.x | gRPC code generation |

### Installation (Ubuntu/Debian)

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Node.js (via nvm)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 20
nvm use 20

# pnpm
npm install -g pnpm

# PostgreSQL
sudo apt install postgresql postgresql-contrib

# Protocol Buffers
sudo apt install protobuf-compiler

# Build essentials
sudo apt install build-essential pkg-config libssl-dev
```

### Installation (macOS)

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Node.js (via nvm)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 20

# pnpm
npm install -g pnpm

# PostgreSQL
brew install postgresql@15
brew services start postgresql@15

# Protocol Buffers
brew install protobuf
```

### Installation (Windows)

```powershell
# Use winget or manual installers
winget install Rustlang.Rustup
winget install OpenJS.NodeJS.LTS
winget install PostgreSQL.PostgreSQL
winget install Google.Protobuf

# pnpm
npm install -g pnpm
```

---

## Project Structure

```
ozone-studio/
├── core/                    # Rust core (single crate)
│   ├── Cargo.toml
│   ├── build.rs            # Proto compilation
│   └── src/
│       ├── main.rs         # Entry point
│       ├── lib.rs          # Library exports
│       ├── bootstrap.rs    # Initialization sequence
│       ├── config.rs       # Configuration loading
│       ├── auth.rs         # Authentication
│       ├── session.rs      # Session management
│       ├── zsei/           # ZSEI implementation
│       ├── pipelines/      # Pipeline system
│       └── grpc/           # gRPC server
│
├── ui/                      # Electron UI
│   ├── package.json
│   ├── electron-builder.json
│   └── src/
│       ├── main/           # Electron main process
│       ├── renderer/       # React renderer
│       └── shared/         # Shared types
│
├── storage/                 # Database schemas
│   ├── migrations/
│   └── schemas/
│
├── config/                  # Configuration files
│   ├── default.toml
│   └── development.toml
│
└── scripts/                 # Build scripts
    ├── build.sh
    ├── dev.sh
    └── package.sh
```

---

## Quick Start

### 1. Clone and Setup

```bash
cd ozone-studio

# Initialize database
createdb ozone_studio
psql ozone_studio < storage/schemas/init.sql

# Install UI dependencies
cd ui && pnpm install && cd ..

# Build core
cd core && cargo build --release && cd ..
```

### 2. Development Mode

```bash
# Terminal 1: Run Rust core
cd core && cargo run

# Terminal 2: Run Electron UI in dev mode
cd ui && pnpm dev
```

Or use the combined script:
```bash
./scripts/dev.sh
```

### 3. Production Build

```bash
./scripts/build.sh
```

---

## Building

### Build Rust Core Only

```bash
cd core
cargo build --release
```

The binary will be at `target/release/ozone-studio-core`

### Build Electron UI Only

```bash
cd ui
pnpm build
```

### Build Complete Application

```bash
./scripts/build.sh
```

This will:
1. Build Rust core in release mode
2. Build Electron UI
3. Package everything together

---

## Packaging for Distribution

### Package for Current Platform

```bash
./scripts/package.sh
```

### Package for Specific Platforms

```bash
# Linux
./scripts/package.sh --linux

# macOS
./scripts/package.sh --mac

# Windows (cross-compile from Linux/Mac)
./scripts/package.sh --win
```

### Output Locations

| Platform | Output |
|----------|--------|
| Linux | `dist/ozone-studio-{version}-linux-x64.AppImage` |
| macOS | `dist/ozone-studio-{version}-mac-arm64.dmg` |
| Windows | `dist/ozone-studio-{version}-win-x64.exe` |

---

## Configuration

### Config File Locations

| Environment | Location |
|-------------|----------|
| Development | `config/development.toml` |
| Production | `~/.config/ozone-studio/config.toml` |

### Configuration Structure

```toml
[general]
data_dir = "~/.ozone-studio"
log_level = "info"

[auth]
key_store_path = "~/.ozone-studio/keys"

[database]
url = "postgresql://localhost/ozone_studio"

[zsei]
global_path = "~/.ozone-studio/zsei/global"
local_path = "~/.ozone-studio/zsei/local"

[network]
enabled = true
dht_bootstrap = ["dht.ozone.network:6881"]
grpc_port = 50051

[ui]
theme = "home-dashboard"
meta_portion_width = 20

[consciousness]
enabled = false

[llm]
# LLM settings are managed by the Prompt Pipeline
# This section can be populated by the Settings pipeline
provider = "local"  # "local", "openai", "anthropic", "custom"
model_path = "~/.ozone-studio/models"
```

---

## Database Setup

### Initialize Database

```bash
# Create database
createdb ozone_studio

# Run migrations
psql ozone_studio < storage/migrations/001_initial_schema.sql
psql ozone_studio < storage/migrations/002_zsei_containers.sql
psql ozone_studio < storage/migrations/003_tasks.sql
```

### Reset Database (Development)

```bash
dropdb ozone_studio
createdb ozone_studio
for f in storage/migrations/*.sql; do psql ozone_studio < "$f"; done
```

---

## Pipeline Development

### Pipeline Location

All built-in pipelines are in `core/src/pipelines/`. Each pipeline is a Rust module implementing the `Pipeline` trait.

### Creating a New Pipeline

1. Create the pipeline file:
```rust
// core/src/pipelines/my_pipeline.rs
use crate::pipelines::{Pipeline, PipelineInput, PipelineOutput, PipelineResult};

pub struct MyPipeline;

impl Pipeline for MyPipeline {
    fn id(&self) -> u64 { 100 } // Unique ID
    
    fn name(&self) -> &str { "my_pipeline" }
    
    fn execute(&self, input: PipelineInput) -> PipelineResult<PipelineOutput> {
        // Implementation
        Ok(PipelineOutput::default())
    }
}
```

2. Register in `core/src/pipelines/mod.rs`:
```rust
mod my_pipeline;
pub use my_pipeline::MyPipeline;

pub fn register_built_in_pipelines(registry: &mut PipelineRegistry) {
    // ...existing pipelines...
    registry.register(Box::new(MyPipeline));
}
```

### Pipeline Categories

| ID Range | Category |
|----------|----------|
| 1-99 | System (auth, theme, ZSEI) |
| 100-199 | Task Management |
| 200-299 | Methodology/Blueprint |
| 300-399 | Analysis (code, text) |
| 400-499 | External (browser, references) |
| 500-599 | UI Themes |
| 1000-1999 | Consciousness |

---

## Testing

### Run All Tests

```bash
# Rust tests
cd core && cargo test

# UI tests
cd ui && pnpm test

# Integration tests
./scripts/test.sh
```

### Run Specific Tests

```bash
# Rust - specific test
cargo test test_zsei_container

# Rust - specific module
cargo test zsei::

# UI - specific test file
cd ui && pnpm test TaskViewer.test.tsx
```

---

## Debugging

### Rust Core

```bash
# Debug build with symbols
cargo build

# Run with logging
RUST_LOG=debug cargo run

# Use lldb/gdb
lldb target/debug/ozone-studio-core
```

### Electron UI

```bash
# Dev mode includes DevTools
cd ui && pnpm dev

# In DevTools: Ctrl+Shift+I (or Cmd+Opt+I on Mac)
```

### gRPC Debugging

```bash
# Install grpcurl
go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest

# List services
grpcurl -plaintext localhost:50051 list

# Call method
grpcurl -plaintext -d '{"pipeline_id": 1}' localhost:50051 ozone.PipelineService/Execute
```

---

## Common Issues

### "Database connection refused"
```bash
# Check PostgreSQL is running
sudo systemctl status postgresql
# or on Mac
brew services list
```

### "gRPC port already in use"
```bash
# Find and kill process
lsof -i :50051
kill -9 <PID>
```

### "Electron window is blank"
```bash
# Clear Electron cache
rm -rf ~/.config/ozone-studio/cache
# or on Mac
rm -rf ~/Library/Application\ Support/ozone-studio/cache
```

### "Rust build fails with protoc error"
```bash
# Ensure protoc is installed
which protoc
protoc --version

# If not found, install it
sudo apt install protobuf-compiler  # Linux
brew install protobuf               # Mac
```

---

## Architecture Notes

### Core Responsibilities

The Rust core ONLY:
- Bootstraps the application
- Authenticates users
- Manages ZSEI containers
- Executes pipelines
- Provides gRPC API to UI
- Manages tasks

### Pipeline Responsibilities

EVERYTHING ELSE is a pipeline:
- LLM calls → Prompt Pipeline
- UI rendering → Theme Pipelines
- Code analysis → CodeAnalysis Pipeline
- Settings management → Settings Pipeline
- Methodology fetching → MethodologyFetch Pipeline

### UI Responsibilities

The Electron UI:
- Renders the chrome (window frame)
- Displays Theme Area (80%)
- Displays Meta Portion (20%)
- Displays Connection Bar (bottom)
- Communicates with core via gRPC

The UI does NOT contain business logic - it renders what pipelines tell it to.

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## Version History

| Version | Date | Notes |
|---------|------|-------|
| 0.3 | 2025-01-17 | Initial complete specification |
