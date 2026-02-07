# OZONE STUDIO — DEPLOYMENT GUIDE v0.4.0

**OZONE STUDIO — Omnidirectional Zero-Shot Neural Engine**  
**A Collective AGI Framework with Optional Consciousness**

---

## TABLE OF CONTENTS

1. [Overview](#1-overview)
2. [Prerequisites](#2-prerequisites)
3. [Build Ozone Studio Backend](#3-build-ozone-studio-backend)
4. [Build llama.cpp (CMake)](#4-build-llamacpp-cmake)
5. [Build BitNet (Optional)](#5-build-bitnet-optional)
6. [Download Models](#6-download-models)
7. [Build Whisper (Voice)](#7-build-whisper-voice)
8. [Configuration (config.toml)](#8-configuration-configtoml)
9. [Build & Run UI](#9-build--run-ui)
10. [First Launch Flow](#10-first-launch-flow)
11. [P2P Network Setup](#11-p2p-network-setup)
12. [Logging & Troubleshooting](#12-logging--troubleshooting)
13. [FAQ](#13-faq)

---

## 1. OVERVIEW

### What Happens on First Run

When you run `ozone-studio` for the first time:

1. **Auto-creates** `config.toml` with sensible defaults
2. **Auto-creates** `zsei_data/` directory structure
3. **Auto-creates** `pipelines/custom/` for user pipelines
4. **Starts** gRPC server on port 50051
5. **Waits** for UI connection

### Directory Structure After Build

```
target/release/
├── ozone-studio              # Main executable
├── config.toml               # Auto-generated (edit after first run)
├── zsei_data/
│   ├── global/               # P2P shared knowledge
│   └── local/
│       └── 0.json            # Root container (auto-created)
└── pipelines/
    └── custom/               # User-created pipelines
```

**Note:** `zsei_data/local/0.json` is the root container. Additional containers (methodologies, blueprints) are created during use.

---

## 2. PREREQUISITES

### System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| RAM | 4 GB | 16+ GB |
| Storage | 10 GB | 50+ GB |
| CPU | 4 cores | 8+ cores |
| GPU | None (CPU works) | NVIDIA CUDA or AMD ROCm |

### Required Software

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install -y \
    build-essential \
    cmake \
    git \
    curl \
    pkg-config \
    libssl-dev \
    libclang-dev \
    nodejs \
    npm \
    protobuf-compiler

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify versions
rustc --version    # 1.75+ required
node --version     # 18+ required  
npm --version      # 9+ required
cmake --version    # 3.14+ required
```

---

## 3. BUILD OZONE STUDIO BACKEND

### Clone Repository

```bash
git clone https://github.com/RebornBeat/Ozone-Studio.git
cd Ozone-Studio
```

### Build Release Binary

```bash
# Standard build
cargo build --release
```

### Verify Build

```bash
ls -la target/release/ozone-studio
# Should show executable ~15-30 MB
```

---

## 4. BUILD LLAMA.CPP (CMAKE)

**IMPORTANT:** The Makefile build is DEPRECATED. Use CMake.

### Build llama.cpp

```bash
# Navigate to llama.cpp directory
cd llama.cpp

# Create build directory
mkdir -p build && cd build

# Configure (pick ONE based on your hardware):

# Option A: CPU-only
cmake .. -DGGML_BLAS=ON

# Option B: NVIDIA GPU (CUDA)
cmake .. -DGGML_CUDA=ON

# Option C: AMD GPU (ROCm)
cmake .. -DGGML_HIPBLAS=ON

# Option D: Apple Silicon (Metal)
cmake .. -DGGML_METAL=ON

# Build
cmake --build . --config Release -j$(nproc)
```

### Install Binaries

```bash
# Option 1: Copy to /usr/local/bin
sudo cp build/bin/llama-cli /usr/local/bin/
sudo cp build/bin/llama-server /usr/local/bin/

# Option 2: Add to PATH (in ~/.bashrc)
export PATH="$HOME/Projects/Ozone-Studio/llama.cpp/build/bin:$PATH"
source ~/.bashrc
```

### Verify Installation

```bash
llama-cli --version
llama-server --version
```

---

## 5. BUILD BITNET (OPTIONAL)

BitNet provides ultra-efficient 1-bit quantized models. Ideal for low-resource systems.

### Build BitNet

```bash
cd BitNet

# Install Python dependencies (for model download script)
pip3 install -r requirements.txt --break-system-packages

# Build inference engine
mkdir -p build && cd build
cmake .. -DGGML_BLAS=ON
cmake --build . --config Release -j$(nproc)
```

### Download BitNet Model

```bash
cd BitNet

# Download BitNet 1.58-bit 2B model (recommended starter)
python3 download_model.py --model bitnet_b1_58-2B-4T --output ~/ozone-models/bitnet/

# Verify
ls -la ~/ozone-models/bitnet/
```

### BitNet Model Options

| Model | Parameters | Size on Disk | RAM Required |
|-------|------------|--------------|--------------|
| bitnet_b1_58-2B-4T | 2B | ~500 MB | 1-2 GB |
| bitnet_b1_58-7B | 7B | ~2 GB | 3-4 GB |

---

## 6. DOWNLOAD MODELS

### Create Models Directory

```bash
mkdir -p ~/ozone-models/{gguf,bitnet,whisper}
```

### GGUF Models (for llama.cpp)

| Model | Size | RAM Required | Context | Best For |
|-------|------|--------------|---------|----------|
| Phi-3 Mini 4K Q4 | 2.4 GB | 4 GB | 4K | General, fast |
| Mistral 7B Q4_K_M | 4.1 GB | 6 GB | 8K | Balanced |
| Llama 3.2 3B | 2.0 GB | 4 GB | 8K | Efficient |
| CodeLlama 7B Q4 | 4.0 GB | 6 GB | 16K | Code tasks |
| Qwen2.5 7B Q4 | 4.3 GB | 6 GB | 32K | Long context |

### Download Example (Mistral 7B)

```bash
cd ~/ozone-models/gguf

# Using wget
wget https://huggingface.co/TheBloke/Mistral-7B-Instruct-v0.2-GGUF/resolve/main/mistral-7b-instruct-v0.2.Q4_K_M.gguf

# Or using huggingface-cli
pip install huggingface_hub
huggingface-cli download TheBloke/Mistral-7B-Instruct-v0.2-GGUF mistral-7b-instruct-v0.2.Q4_K_M.gguf --local-dir .
```

### Context Length Per Model

**IMPORTANT:** Each model has its own context length limit. Ozone Studio tracks this per-model:

```toml
# In config.toml, each model specifies its context_length
[[models.available_models]]
name = "Mistral 7B"
model_type = "gguf"
identifier = "~/ozone-models/gguf/mistral-7b-instruct-v0.2.Q4_K_M.gguf"
context_length = 8192

[[models.available_models]]
name = "Qwen2.5 7B"
model_type = "gguf"
identifier = "~/ozone-models/gguf/qwen2.5-7b-instruct-q4_k_m.gguf"
context_length = 32768

[[models.available_models]]
name = "Claude Sonnet (API)"
model_type = "api"
identifier = "claude-sonnet-4-20250514"
context_length = 200000
```

When you switch models in the UI, Ozone Studio uses that model's specific context length.

---

## 7. BUILD WHISPER (VOICE)

### Option A: whisper.cpp (Standalone)

```bash
# Clone
git clone https://github.com/ggerganov/whisper.cpp.git
cd whisper.cpp

# Build
mkdir -p build && cd build
cmake ..
cmake --build . --config Release -j$(nproc)

# Download model
cd ..
bash ./models/download-ggml-model.sh base.en

# Install
sudo cp build/bin/whisper-cli /usr/local/bin/
```

### Option B: whisper-rs (Integrated - Default)

whisper-rs is already compiled into Ozone Studio. Just download the model:

```bash
# Download whisper model
mkdir -p ~/ozone-models/whisper
cd ~/ozone-models/whisper

wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
```

### Whisper Model Options

| Model | Size | Speed | Quality |
|-------|------|-------|---------|
| ggml-tiny.en.bin | 75 MB | Fastest | Basic |
| ggml-base.en.bin | 142 MB | Fast | Good (recommended) |
| ggml-small.en.bin | 466 MB | Medium | Better |
| ggml-medium.en.bin | 1.5 GB | Slow | Best |

---

## 8. CONFIGURATION (config.toml)

### Auto-Generated Config

On first run, `ozone-studio` creates `config.toml`. You can then edit it:

```bash
cd target/release
./ozone-studio  # Creates config.toml
# Press Ctrl+C to stop
nano config.toml  # Edit as needed
```

### Full Configuration Reference

```toml
# OZONE STUDIO CONFIGURATION
# Generated automatically, customize as needed

[general]
data_dir = "zsei_data"
log_level = "info"  # trace, debug, info, warn, error

[zsei]
global_path = "zsei_data/global"
local_path = "zsei_data/local"
cache_path = "zsei_data/cache"
max_containers_in_memory = 10000
mmap_enabled = true

# IMPORTANT: ZSEI uses ZERO-SHOT semantic traversal
# No external embedding model required - uses the same LLM
# This field is for optional hybrid mode only
embedding_dimension = 384

[pipelines]
builtin_path = "pipelines"
custom_path = "pipelines/custom"
max_concurrent_pipelines = 10

[tasks]
max_queued_tasks = 100
task_timeout_secs = 3600
preserve_completed_tasks = true
max_task_history = 1000

[auth]
keystore_path = "zsei_data/keystore"
session_duration_secs = 86400
challenge_expiry_secs = 300

[integrity]
enabled = true
check_interval_secs = 3600
rollback_path = "zsei_data/integrity/rollback"
max_versions = 100

[network]
enable_p2p = true
enable_cloud_sync = false
p2p_port = 9090
max_peers = 50
enable_mdns = true  # Auto-discover on local network
batch_sync_interval_secs = 60

# Bootstrap nodes (add after genesis peer is established)
bootstrap_nodes = [
    # "/ip4/YOUR_IP/tcp/9090/p2p/YOUR_PEER_ID"
]

[grpc]
address = "127.0.0.1"
port = 50051

[ui]
theme = "home_dashboard"
default_tabs = ["workspace", "library", "settings"]
meta_portion_width_percent = 20

[consciousness]
enabled = false
emotional_system_enabled = true
experience_memory_enabled = true
i_loop_interval_ms = 60000

[models]
# Primary model type: "gguf", "bitnet", "api"
model_type = "gguf"

# API settings (when model_type = "api")
api_endpoint = "https://api.anthropic.com/v1/messages"
api_key_env = "ANTHROPIC_API_KEY"
api_model = "claude-sonnet-4-20250514"

# Local model settings (when model_type = "gguf" or "bitnet")
local_model_path = ""  # Set via UI or edit here
context_length = 8192  # Default, overridden per-model

# GPU acceleration (0 = CPU only)
gpu_layers = 0

# llama-server settings
llama_server_port = 8080  # Internal server port
llama_server_host = "127.0.0.1"

allow_user_selection = true

# Available models for UI selection
[[models.available_models]]
name = "Mistral 7B Q4"
model_type = "gguf"
identifier = "~/ozone-models/gguf/mistral-7b-instruct-v0.2.Q4_K_M.gguf"
context_length = 8192

[[models.available_models]]
name = "BitNet 1.58 2B"
model_type = "bitnet"
identifier = "~/ozone-models/bitnet/bitnet_b1_58-2B-4T"
context_length = 4096

[[models.available_models]]
name = "Claude Sonnet (API)"
model_type = "api"
identifier = "claude-sonnet-4-20250514"
context_length = 200000

[voice]
# Voice backend: "whisper_rs" (integrated), "whisper_cpp" (standalone), "api"
backend = "whisper_rs"
whisper_model_path = "~/ozone-models/whisper/ggml-base.en.bin"
# If using whisper_cpp standalone:
whisper_cpp_path = "/usr/local/bin/whisper-cli"
```

### ZSEI Zero-Shot Clarification

**IMPORTANT:** ZSEI was designed for **ZERO-SHOT semantic traversal**:

- ✅ Uses the LLM itself for semantic understanding
- ✅ No external embedding model required by default
- ✅ The `embedding_dimension` field is for optional hybrid mode
- ✅ Zero-shot means: same model does both understanding AND generation

This is what makes ZSEI unique compared to traditional RAG systems.

---

## 9. BUILD & RUN UI

### Install Dependencies

```bash
cd ui
npm install
```

### Development Mode (Hot Reload)

```bash
# Terminal 1: Start backend
cd target/release
./ozone-studio

# Terminal 2: Start UI
cd ui
npm run electron
```

### Production Build

```bash
cd ui
npm run electron:build
```

### Run Production App

```bash
# Linux AppImage (portable, no install)
chmod +x "dist/Ozone Studio-0.3.0.AppImage"
./dist/Ozone\ Studio-0.3.0.AppImage

# Or install .deb
sudo dpkg -i dist/ozone-studio_0.3.0_amd64.deb
```

---

## 10. FIRST LAUNCH FLOW

### What Happens Automatically

When you start `ozone-studio`:

1. **Creates config.toml** if not exists
2. **Creates zsei_data/** directories
3. **Creates pipelines/custom/** directory
4. **Generates P2P peer identity** (stored in zsei_data/keystore)
5. **Starts gRPC server** on configured port
6. **Starts llama-server** automatically if local model selected
7. **Waits for UI** to connect

### First-Time Setup Flow in UI

When the UI connects for the first time:

1. **Setup Wizard** appears (if no model configured)
2. **Select Model Type:**
   - API (enter API key)
   - Local GGUF (browse to .gguf file)
   - BitNet (browse to BitNet model)
3. **Configure Voice** (optional)
4. **Enable Consciousness** (optional)
5. **Done!** Home dashboard appears

### Automatic LLM Server Management

When using local models, Ozone Studio handles everything:

```
User selects GGUF model in UI
         ↓
Backend receives model change request
         ↓
Backend stops any existing llama-server
         ↓
Backend spawns: llama-server -m <model_path> --port 8080 --ctx-size <context>
         ↓
Backend polls http://127.0.0.1:8080/health until ready
         ↓
Backend routes LLM requests to local server
         ↓
User gets response (no manual server needed!)
```

**No bash scripts required** - everything is automatic.

---

## 11. P2P NETWORK SETUP

### How P2P Works

1. **Local Discovery (mDNS):** Automatically finds peers on same network
2. **Bootstrap Nodes:** Initial peers to connect to for wider network
3. **Peer Exchange:** Connected peers share their peer lists

### Genesis Node (First User)

If you're starting the network:

```toml
# config.toml
[network]
enable_p2p = true
enable_mdns = true
bootstrap_nodes = []  # Empty - you ARE the genesis
```

Run Ozone Studio and note your Peer ID from logs:

```bash
./ozone-studio 2>&1 | grep -i "peer"
# Output: Local peer ID: 12D3KooWxxxxxx...
# Output: Listening on /ip4/192.168.1.100/tcp/9090
```

Share this with other users as a bootstrap node.

### Subsequent Users

Add the genesis node (or any known peer) to bootstrap:

```toml
[network]
bootstrap_nodes = [
    "/ip4/192.168.1.100/tcp/9090/p2p/12D3KooWxxxxxx..."
]
```

### Public Bootstrap Nodes

Once established, bootstrap nodes can be published at:
- GitHub: `https://raw.githubusercontent.com/RebornBeat/Ozone-Studio/main/BOOTSTRAP_PEERS.txt`
- DNS TXT record: `_ozone-bootstrap.ozonestudio.xyz`

---

## 12. LOGGING & TROUBLESHOOTING

### Enable Debug Logging

```bash
# Method 1: Environment variable
RUST_LOG=debug ./ozone-studio

# Method 2: config.toml
[general]
log_level = "debug"
```

### Save Logs to File

```bash
# Save all output
./ozone-studio 2>&1 | tee ozone.log

# With timestamps
./ozone-studio 2>&1 | ts '[%Y-%m-%d %H:%M:%S]' | tee ozone.log
```

### UI Logs

**Development Mode:** Logs in terminal running `npm run electron`

**Production Mode:** 
- Linux: `~/.config/ozone-studio/logs/`
- macOS: `~/Library/Logs/ozone-studio/`
- Windows: `%APPDATA%/ozone-studio/logs/`

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| `llama-server not found` | Not in PATH | Build llama.cpp with CMake, add to PATH |
| `Connection refused 50051` | Backend not running | Start `./ozone-studio` first |
| `UI shows Offline` | gRPC not connecting | Check backend running, port 50051 open |
| `CUDA out of memory` | Model too large for GPU | Reduce `gpu_layers` or use smaller model |
| `Model loading failed` | Wrong path | Check `local_model_path` in config |
| `Peer connection failed` | Firewall | Open UDP/TCP port 9090 |

### Health Checks

```bash
# Backend health
curl -s http://127.0.0.1:50051/health || echo "Backend not running"

# llama-server health (if using local model)
curl -s http://127.0.0.1:8080/health || echo "LLM server not running"

# Check if ports are in use
ss -tlnp | grep -E "50051|8080|9090"
```

---

## 13. FAQ

### Q: Why is `zsei_data/` empty after build?

**A:** `zsei_data/` in `target/release/` is a runtime directory. It starts with just the root container (`0.json`). Methodologies, blueprints, and knowledge accumulate during use.

### Q: Do I need an embedding model?

**A:** No! ZSEI uses **zero-shot semantic traversal**. The same LLM you use for generation also handles semantic understanding. No separate embedding model needed.

### Q: Can I use multiple models?

**A:** Yes! Configure multiple models in `[[models.available_models]]` and switch between them in the UI. Each can have its own `context_length`.

### Q: How do I update the bootstrap nodes list?

**A:** Edit `config.toml` → `[network]` → `bootstrap_nodes`. Changes take effect on next restart.

### Q: Is my data synced automatically?

**A:** With P2P enabled:
- `zsei_data/global/` syncs with connected peers (shared knowledge)
- `zsei_data/local/` stays local (your private data)

### Q: How do I run without P2P?

**A:** Set `enable_p2p = false` in config.toml. Works fully offline.

---

## QUICK REFERENCE

### Build Commands

```bash
# Backend
cargo build --release

# llama.cpp
cd llama.cpp && mkdir build && cd build
cmake .. -DGGML_CUDA=ON  # or other options
cmake --build . --config Release -j$(nproc)

# BitNet
cd BitNet && mkdir build && cd build
cmake .. && cmake --build . -j$(nproc)

# UI
cd ui && npm install && npm run electron:build
```

### Run Commands

```bash
# Development
./target/release/ozone-studio &
cd ui && npm run electron

# Production
./target/release/ozone-studio &
./ui/dist/Ozone\ Studio-0.3.0.AppImage
```

### Essential Paths

| Path | Purpose |
|------|---------|
| `target/release/ozone-studio` | Main executable |
| `target/release/config.toml` | Configuration |
| `target/release/zsei_data/` | Runtime data |
| `~/ozone-models/` | Downloaded models |

---

**Document Version:** 0.3.1  
**Last Updated:** 2025-02-03  
**Author:** Christian Liz-Fonts
