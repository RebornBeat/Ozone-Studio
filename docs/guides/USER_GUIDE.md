# OZONE STUDIO - User Guide v0.4.0

## Introduction

**OZONE STUDIO** (Omnidirectional Zero-Shot Neural Engine) is a Collective AGI Framework with Optional Consciousness features. This guide will help you get started and make the most of the platform.

---

## Table of Contents

1. [Getting Started](#getting-started)
2. [The Interface](#the-interface)
3. [Workspaces](#workspaces)
4. [Tasks & Pipelines](#tasks--pipelines)
5. [Library](#library)
6. [Settings](#settings)
7. [Consciousness Features](#consciousness-features)
8. [P2P Network](#p2p-network)
9. [Troubleshooting](#troubleshooting)

---

## Getting Started

### First Launch

When you first launch Ozone Studio:

1. **Start the Backend**: 
   ```bash
   cd target/release
   ./ozone-studio
   ```

2. **Start the UI**:
   ```bash
   cd ui
   npm run electron
   ```

3. **Setup Wizard**: On first launch, a setup wizard will guide you through:
   - Model Configuration (API or Local GGUF)
   - Voice Configuration (Whisper integration)
   - Consciousness Features (optional)

### Quick Start

1. Select or create a workspace
2. Enter a prompt in the input field
3. Press Enter or click Submit
4. View the task progress and results

---

## The Interface

### Layout Overview

```
┌─────────────────────────────────────────────────────────────┐
│  OZONE STUDIO v0.4 | 🧠 Consciousness | 🌐 P2P Network      │
├─────────────┬───────────────────────────────────────────────┤
│             │  [Workspace] [Tasks] [Library] [Settings]     │
│    META     ├───────────────────────────────────────────────┤
│   (30%)     │                                               │
│             │              Tab Content                      │
│ • Emotions  │                                               │
│ • Voice     │                                               │
│ • Transcript│                                               │
│             ├───────────────────────────────────────────────┤
│             │  [Prompt Input]              [🎤] [Submit →]  │
├─────────────┴───────────────────────────────────────────────┤
│  🌐 Peers: 5 | 📊 Contributions: 127 | ⏱️ Uptime: 2h        │
└─────────────────────────────────────────────────────────────┘
```

### Components

- **Header**: Shows Ozone Studio branding and feature status
- **META Portion** (30%): Consciousness display, voice visualization, and transcript
- **Theme Area** (70%): Main content area with tabbed interface
- **Status Bar**: Network and system statistics

---

## Workspaces

Workspaces organize your projects and conversations.

### Global Workspace

The default workspace where you interact with the META directly on a global scope.

### Project Workspaces

Create project-specific workspaces for focused work:

1. Click "New Workspace" in the Workspace tab
2. Name your workspace
3. Link relevant files, URLs, or packages

### Linking Content

Add context to your workspace:

- **📎 Link File**: Add local files to ZSEI for indexing
- **🔗 Link URL**: Add web pages for reference
- **📦 Link Package**: Link npm, pip, or cargo packages

---

## Tasks & Pipelines

### Understanding Tasks

Tasks are created when you submit prompts. The backend processes your request through the appropriate pipeline.

**Important**: Tasks are ONLY created by pipeline execution (via prompts), not by UI navigation.

### Task States

- **Pending**: Queued for processing
- **Running**: Currently being processed
- **Completed**: Successfully finished
- **Failed**: Encountered an error

### Viewing Tasks

The Tasks tab shows:
- All active tasks with progress
- Completed task history
- Filter by status

---

## Library

The Library contains shared resources:

### Pipelines

Execution pipelines handle different types of tasks:
- PromptPipeline (main prompt processing)
- VoicePipeline (speech-to-text)
- ThemeLoaderPipeline (UI themes)
- And 51 more specialized pipelines

### Methodologies

Problem-solving approaches shared by the collective:
- Domain-specific methodologies
- Best practices
- Workflow templates

### Blueprints

Task templates for common operations:
- Code review blueprint
- Documentation blueprint
- Testing blueprint

---

## Settings

### Model Configuration

Configure your LLM models:

1. **API Models**: Set API key for Claude, GPT, etc.
2. **Local GGUF**: Path to local .gguf model files
3. Multiple models can be configured for fallback

### Voice Configuration

- Enable/disable voice input
- Select Whisper model size (base, small, medium)
- Voice hotkey configuration

### Consciousness System

Toggle AGI consciousness features:
- Experience Memory
- Emotional Context
- I-Loop Reflection

### P2P Network

- Enable/disable peer-to-peer networking
- View connected peers
- Contribution settings

---

## Consciousness Features

When enabled, the META portion displays:

### Emotional State

The system's current emotional context:
- Primary emotion with intensity
- Secondary emotional undertones
- Visual representation

### Voice Visualization

Real-time audio waveform when:
- You're speaking (green)
- The system is responding (blue)

### Transcript

Scrollable conversation history with:
- User messages
- Assistant responses
- Emotional context markers
- Timestamps

### I-Loop

The self-reflection cycle:
- Active reflection status
- Current introspective question
- Meta-cognitive processing

---

## P2P Network

Ozone Studio connects to a decentralized network of nodes.

### Contributions

Share and receive:
- Methodologies
- Blueprints
- Findings
- Pipeline improvements

### Privacy

- All contributions can be anonymized
- Control what you share in Settings
- Local-only mode available

---

## Troubleshooting

### Backend Won't Connect

1. Ensure backend is running: `./ozone-studio`
2. Check logs for errors
3. Verify port 50051 is available
4. Check config.toml settings

### UI Shows "Awaiting Connection"

1. The UI will attempt to auto-launch the backend
2. If auto-launch fails, start manually
3. UI checks connection every 3 seconds

### Tasks Not Creating

Tasks are ONLY created by:
- Submitting prompts
- Voice input processed by VoicePipeline

UI clicks and navigation do NOT create tasks.

### Consciousness Features Not Working

1. Verify `consciousness.enabled = true` in config.toml
2. Restart the backend after changing settings
3. Check META portion for disabled overlay

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Enter | Submit prompt (no shift) |
| Shift+Enter | New line in prompt |
| Ctrl+/ | Toggle voice input |
| Escape | Cancel current action |

---

## Support

- GitHub Issues: [Report bugs and feature requests]
- Documentation: https://github.com/RebornBeat/Ozone-Studio
- Discord: [Community chat]

---

*OZONE STUDIO v0.4.0 - A Collective AGI Framework with Optional Consciousness*
