# Ozone Studio — Deployment Guide

## Overview

This guide covers deploying Ozone Studio across supported platforms. Ozone Studio is a pipeline execution engine that can run as a desktop application (via Electron) with a Rust backend.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Build from Source](#build-from-source)
3. [Pre-built Binaries](#pre-built-binaries)
4. [Platform-Specific Instructions](#platform-specific-instructions)
5. [First Run Configuration](#first-run-configuration)
6. [Network Configuration](#network-configuration)
7. [Database Setup](#database-setup)
8. [Verification](#verification)
9. [Troubleshooting Deployment](#troubleshooting-deployment)

---

## 1. Prerequisites

### System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| CPU | 4 cores | 8+ cores |
| RAM | 8 GB | 16+ GB |
| Storage | 20 GB SSD | 100+ GB SSD |
| GPU | Not required | CUDA-capable (for local LLMs) |

### Software Requirements

**All Platforms:**
- PostgreSQL 15+
- Node.js 20+ LTS
- Rust 1.75+ (if building from source)

**Linux (Ubuntu 22.04+):**
```bash
# Essential packages
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev \
    libpq-dev postgresql postgresql-contrib \
    nodejs npm

# Rust (if building from source)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**macOS:**
```bash
# Using Homebrew
brew install postgresql node rust

# Start PostgreSQL
brew services start postgresql
```

**Windows:**
- Install PostgreSQL from [postgresql.org](https://www.postgresql.org/download/windows/)
- Install Node.js from [nodejs.org](https://nodejs.org/)
- Install Rust from [rustup.rs](https://rustup.rs/)
- Install Visual Studio Build Tools (for Rust compilation)

---

## 2. Build from Source

### Clone Repository

```bash
git clone https://github.com/ozone-studio/ozone-studio.git
cd ozone-studio
```

### Build Rust Core

```bash
# Build all crates in release mode
cargo build --release

# This creates binaries in ./target/release/
```

### Build Electron UI

```bash
cd ui

# Install dependencies
npm install

# Build for production
npm run build

# Package for your platform
npm run package
```

### Combined Build Script

```bash
# From project root
./scripts/build.sh --release

# Options:
#   --release     Production build
#   --dev         Development build with debug symbols
#   --platform    Specific platform (linux, macos, windows)
```

---

## 3. Pre-built Binaries

Download pre-built binaries from the releases page:

```
https://github.com/ozone-studio/ozone-studio/releases
```

### Available Packages

| Platform | Architecture | Package |
|----------|--------------|---------|
| Linux | x64 | ozone-studio-linux-x64.tar.gz |
| Linux | ARM64 | ozone-studio-linux-arm64.tar.gz |
| macOS | x64 | ozone-studio-macos-x64.dmg |
| macOS | ARM64 (M1/M2) | ozone-studio-macos-arm64.dmg |
| Windows | x64 | ozone-studio-windows-x64.exe |

### Extract and Install

**Linux:**
```bash
tar -xzf ozone-studio-linux-x64.tar.gz
cd ozone-studio
./install.sh
```

**macOS:**
```bash
# Mount DMG and drag to Applications
# Or via command line:
hdiutil attach ozone-studio-macos-arm64.dmg
cp -r "/Volumes/Ozone Studio/Ozone Studio.app" /Applications/
hdiutil detach "/Volumes/Ozone Studio"
```

**Windows:**
```
Run ozone-studio-windows-x64.exe installer
Follow installation wizard
```

---

## 4. Platform-Specific Instructions

### Linux Installation

```bash
# Create installation directory
sudo mkdir -p /opt/ozone-studio
sudo chown $USER:$USER /opt/ozone-studio

# Extract
tar -xzf ozone-studio-linux-x64.tar.gz -C /opt/ozone-studio

# Create data directories
mkdir -p ~/.ozone-studio/{zsei,models,config}

# Create systemd service (optional, for background execution)
sudo tee /etc/systemd/system/ozone-studio.service > /dev/null <<EOF
[Unit]
Description=Ozone Studio Core Service
After=network.target postgresql.service

[Service]
Type=simple
User=$USER
WorkingDirectory=/opt/ozone-studio
ExecStart=/opt/ozone-studio/ozone-core
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

# Enable and start (if using as service)
sudo systemctl enable ozone-studio
sudo systemctl start ozone-studio
```

### macOS Installation

```bash
# After dragging to Applications

# Create data directories
mkdir -p ~/Library/Application\ Support/OzoneStudio/{zsei,models,config}

# Grant necessary permissions (in System Preferences)
# - Full Disk Access (for file analysis)
# - Accessibility (for voice features)
```

### Windows Installation

```powershell
# After running installer

# Create data directories (PowerShell as Admin)
New-Item -ItemType Directory -Force -Path "$env:APPDATA\OzoneStudio\zsei"
New-Item -ItemType Directory -Force -Path "$env:APPDATA\OzoneStudio\models"
New-Item -ItemType Directory -Force -Path "$env:APPDATA\OzoneStudio\config"

# Add to PATH (optional)
$env:Path += ";C:\Program Files\Ozone Studio"
```

---

## 5. First Run Configuration

### Generate Key Pair

On first run, Ozone Studio will prompt to create or import a key pair:

```
╔══════════════════════════════════════════════════════╗
║          OZONE STUDIO — FIRST RUN SETUP              ║
╠══════════════════════════════════════════════════════╣
║                                                      ║
║  Welcome to Ozone Studio!                            ║
║                                                      ║
║  [1] Generate new key pair                           ║
║  [2] Import existing key pair                        ║
║                                                      ║
║  Your key pair is used for:                          ║
║  • Authentication                                    ║
║  • Signing contributions                             ║
║  • Encrypting private data                           ║
║                                                      ║
╚══════════════════════════════════════════════════════╝
```

### Initial Configuration

After key pair setup, configure basic settings:

**config.toml:**
```toml
[general]
data_dir = "~/.ozone-studio"
log_level = "info"

[database]
host = "localhost"
port = 5432
name = "ozone_studio"
user = "ozone"
password = ""  # Set via environment variable

[network]
enabled = true
dht_bootstrap = ["dht.ozone-studio.io:6881"]
max_peers = 50

[consciousness]
enabled = false  # Enable after initial setup

[llm]
default_provider = "local"  # or "api"
local_model_path = "~/.ozone-studio/models"
```

### Environment Variables

Set sensitive values via environment:

```bash
# Linux/macOS
export OZONE_DB_PASSWORD="your_secure_password"
export OZONE_API_KEY="your_llm_api_key"  # If using API-based LLM

# Windows (PowerShell)
$env:OZONE_DB_PASSWORD = "your_secure_password"
$env:OZONE_API_KEY = "your_llm_api_key"
```

---

## 6. Network Configuration

### Firewall Rules

**Required Ports:**

| Port | Protocol | Purpose |
|------|----------|---------|
| 6881 | TCP/UDP | DHT (peer discovery) |
| 50051 | TCP | gRPC (inter-process) |
| 5432 | TCP | PostgreSQL (local only) |

**Linux (ufw):**
```bash
sudo ufw allow 6881/tcp
sudo ufw allow 6881/udp
```

**Windows (PowerShell as Admin):**
```powershell
New-NetFirewallRule -DisplayName "Ozone Studio DHT" -Direction Inbound -Protocol TCP -LocalPort 6881 -Action Allow
New-NetFirewallRule -DisplayName "Ozone Studio DHT UDP" -Direction Inbound -Protocol UDP -LocalPort 6881 -Action Allow
```

### NAT Traversal

If behind NAT, enable UPnP or configure port forwarding:

```toml
# config.toml
[network]
upnp_enabled = true
# Or specify external port manually:
# external_port = 6881
```

---

## 7. Database Setup

### Create Database and User

```bash
# Connect as postgres superuser
sudo -u postgres psql

# Create user and database
CREATE USER ozone WITH PASSWORD 'your_secure_password';
CREATE DATABASE ozone_studio OWNER ozone;
GRANT ALL PRIVILEGES ON DATABASE ozone_studio TO ozone;

# Enable required extensions
\c ozone_studio
CREATE EXTENSION IF NOT EXISTS vector;  # For embeddings
CREATE EXTENSION IF NOT EXISTS pg_trgm;  # For fuzzy text search

\q
```

### Run Migrations

```bash
# From ozone-studio directory
./ozone-core migrate

# Or manually:
psql -U ozone -d ozone_studio -f storage/migrations/001_initial_schema.sql
psql -U ozone -d ozone_studio -f storage/migrations/002_zsei_containers.sql
psql -U ozone -d ozone_studio -f storage/migrations/003_tasks.sql
# ... continue for all migrations
```

### Initialize ZSEI Storage

```bash
# Initialize binary storage files
./ozone-core init-zsei

# This creates:
# ~/.ozone-studio/zsei/global/containers.bin
# ~/.ozone-studio/zsei/global/children.bin
# ~/.ozone-studio/zsei/global/parents.bin
# ~/.ozone-studio/zsei/local/...
```

---

## 8. Verification

### Health Check

```bash
# Run health check
./ozone-core health

# Expected output:
# ✓ Database connection: OK
# ✓ ZSEI storage: OK
# ✓ gRPC server: OK
# ✓ Network (DHT): OK
# ✓ Pipeline registry: OK (38 pipelines loaded)
```

### Test UI Launch

```bash
# Launch Electron UI
./ozone-studio

# Or on macOS:
open "/Applications/Ozone Studio.app"
```

### Verify First Sync

On first network connection, Ozone Studio will sync with the distributed network:

```
Syncing with network...
├── Downloading modality definitions... (12/12)
├── Downloading category structure... (847/847)
├── Downloading methodology index... (45,623 entries)
├── Downloading blueprint index... (234,102 entries)
└── Downloading pipeline library... (12,345 entries)

Sync complete! Language context version: 2847
```

---

## 9. Troubleshooting Deployment

### Common Issues

**Issue: Database connection failed**
```
Error: Connection refused (os error 111)
```
Solution:
```bash
# Check PostgreSQL is running
sudo systemctl status postgresql

# Check connection settings
psql -U ozone -h localhost -d ozone_studio
```

**Issue: Permission denied on ZSEI files**
```
Error: Permission denied: ~/.ozone-studio/zsei/global/containers.bin
```
Solution:
```bash
# Fix ownership
chown -R $USER:$USER ~/.ozone-studio
chmod -R 755 ~/.ozone-studio
```

**Issue: Network sync stuck**
```
Syncing with network... (0% for > 5 minutes)
```
Solution:
```bash
# Check firewall
sudo ufw status

# Check internet connectivity
ping dht.ozone-studio.io

# Try alternate bootstrap nodes
# Edit config.toml:
[network]
dht_bootstrap = ["dht2.ozone-studio.io:6881", "dht3.ozone-studio.io:6881"]
```

**Issue: Electron fails to launch on Linux**
```
Error: The SUID sandbox helper binary was found...
```
Solution:
```bash
# Option 1: Disable sandbox (less secure)
./ozone-studio --no-sandbox

# Option 2: Fix sandbox permissions
sudo chown root:root chrome-sandbox
sudo chmod 4755 chrome-sandbox
```

### Log Locations

| Platform | Log Location |
|----------|--------------|
| Linux | `~/.ozone-studio/logs/` |
| macOS | `~/Library/Logs/OzoneStudio/` |
| Windows | `%APPDATA%\OzoneStudio\logs\` |

### Getting Help

- GitHub Issues: https://github.com/ozone-studio/ozone-studio/issues
- Documentation: https://docs.ozone-studio.io
- Community Discord: https://discord.gg/ozone-studio

---

## Next Steps

After successful deployment:

1. Review [Configuration Reference](./CONFIGURATION_REFERENCE.md) for all options
2. Complete [First Run Wizard](#first-run-configuration) in the UI
3. Register additional devices if desired
4. Consider enabling [Consciousness Mode](./MIGRATION_GUIDE.md) after familiarization

---

*Document Version: 0.3*
*Last Updated: 2025-01-17*
