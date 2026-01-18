# Ozone Studio — Configuration Reference

## Overview

This document provides a complete reference for all configuration options in Ozone Studio. Configuration is managed through TOML files and environment variables.

---

## Table of Contents

1. [Configuration Files](#configuration-files)
2. [General Settings](#general-settings)
3. [Database Configuration](#database-configuration)
4. [ZSEI Configuration](#zsei-configuration)
5. [Pipeline Configuration](#pipeline-configuration)
6. [Task Configuration](#task-configuration)
7. [Network Configuration](#network-configuration)
8. [LLM Configuration](#llm-configuration)
9. [UI Configuration](#ui-configuration)
10. [Consciousness Configuration](#consciousness-configuration)
11. [Security Configuration](#security-configuration)
12. [Logging Configuration](#logging-configuration)
13. [Environment Variables](#environment-variables)
14. [Complete Example](#complete-example)

---

## 1. Configuration Files

### File Locations

| Platform | Config Directory |
|----------|-----------------|
| Linux | `~/.ozone-studio/config/` |
| macOS | `~/Library/Application Support/OzoneStudio/config/` |
| Windows | `%APPDATA%\OzoneStudio\config\` |

### File Hierarchy

```
config/
├── config.toml        # Main configuration
├── consciousness.toml # Consciousness-specific settings
├── pipelines.toml     # Custom pipeline configurations
└── local.toml         # Local overrides (not synced)
```

### Load Order

1. `default.toml` (bundled with application)
2. `config.toml` (user configuration)
3. `local.toml` (local overrides)
4. Environment variables (highest priority)

---

## 2. General Settings

```toml
[general]
# Application name (used in logs and UI)
app_name = "Ozone Studio"

# Data directory for all storage
# Supports: absolute path, ~ for home, environment variables
data_dir = "~/.ozone-studio"

# Logging level: trace, debug, info, warn, error
log_level = "info"

# Enable telemetry (anonymous usage statistics)
telemetry_enabled = false

# Language for UI and messages
language = "en"

# Timezone (auto-detect if empty)
timezone = ""

# Auto-update settings
[general.updates]
enabled = true
check_interval_hours = 24
auto_install = false
channel = "stable"  # stable, beta, nightly
```

---

## 3. Database Configuration

```toml
[database]
# PostgreSQL connection settings
host = "localhost"
port = 5432
name = "ozone_studio"
user = "ozone"
# Password should be set via OZONE_DB_PASSWORD environment variable
password = ""

# Connection pool settings
[database.pool]
min_connections = 2
max_connections = 20
connection_timeout_seconds = 30
idle_timeout_seconds = 600

# SSL settings
[database.ssl]
enabled = false
mode = "prefer"  # disable, allow, prefer, require, verify-ca, verify-full
ca_cert_path = ""
client_cert_path = ""
client_key_path = ""
```

---

## 4. ZSEI Configuration

```toml
[zsei]
# Storage paths
global_storage_path = "~/.ozone-studio/zsei/global"
local_storage_path = "~/.ozone-studio/zsei/local"

# Memory mapping settings
[zsei.mmap]
enabled = true
preload_indices = true
max_mapped_memory_gb = 4

# Traversal settings
[zsei.traversal]
default_max_depth = 10
default_max_results = 100
cache_hot_paths = true
hot_path_cache_size = 1000

# Integrity settings
[zsei.integrity]
enabled = true
auto_repair = true
verification_interval_hours = 24
max_versions_retained = 10

# Embedding settings
[zsei.embeddings]
enabled = true
model = "all-MiniLM-L6-v2"  # or path to custom model
dimension = 384
index_type = "hnsw"  # hnsw, ivf, flat

[zsei.embeddings.hnsw]
ef_construction = 200
m = 16
ef_search = 100
```

---

## 5. Pipeline Configuration

```toml
[pipeline]
# Pipeline library settings
library_path = "~/.ozone-studio/pipelines"
auto_update = true

# Execution settings
[pipeline.execution]
max_concurrent_pipelines = 4
default_timeout_seconds = 300
retry_failed = true
max_retries = 3

# Built-in pipeline settings
[pipeline.built_in]
# Enable/disable specific built-in pipelines
auth_enabled = true
theme_loader_enabled = true
zsei_query_enabled = true
zsei_write_enabled = true
prompt_enabled = true
voice_enabled = true
methodology_enabled = true
blueprint_enabled = true
code_analysis_enabled = true
text_analysis_enabled = true
browser_navigation_enabled = true

# Zero-shot simulation settings
[pipeline.zero_shot]
default_max_iterations = 10
confidence_threshold = 0.95
early_stop = true
```

---

## 6. Task Configuration

```toml
[task]
# Task management settings
max_queued_tasks = 100
max_history_days = 30
preserve_execution_state = true

# Resource limits per task
[task.limits]
max_cpu_percent = 80
max_memory_mb = 4096
max_duration_seconds = 3600
max_disk_write_mb = 1024

# Scheduling settings
[task.scheduler]
algorithm = "priority"  # priority, fifo, fair
priority_weight_urgency = 0.4
priority_weight_user = 0.3
priority_weight_resource = 0.3

# Execution environment
[task.environment]
isolation = "process"  # process, container, none
cleanup_on_complete = true
preserve_logs = true
```

---

## 7. Network Configuration

```toml
[network]
# Enable distributed features
enabled = true

# DHT (Distributed Hash Table) settings
[network.dht]
enabled = true
bootstrap_nodes = [
    "dht.ozone-studio.io:6881",
    "dht2.ozone-studio.io:6881"
]
port = 6881
max_peers = 50

# NAT traversal
[network.nat]
upnp_enabled = true
external_port = 0  # 0 = auto

# Sync settings
[network.sync]
enabled = true
auto_sync = true
sync_interval_minutes = 15
max_concurrent_downloads = 5
bandwidth_limit_kbps = 0  # 0 = unlimited

# Contribution settings
[network.contribution]
share_methodologies = true
share_blueprints = true
share_pipelines = true
share_experiences = false  # Requires consciousness
anonymize_contributions = true

# Consensus settings
[network.consensus]
participation_enabled = true
min_verification_peers = 3
acceptance_threshold = 0.67
proposal_cooldown_hours = 24
```

---

## 8. LLM Configuration

```toml
[llm]
# Default provider: "local" or "api"
default_provider = "local"

# Local model settings
[llm.local]
enabled = true
model_directory = "~/.ozone-studio/models/llm"

# Supported formats: gguf, onnx, bitnet
default_format = "gguf"

# Model selection (filename in model_directory)
default_model = ""  # Empty = prompt user to select

# Inference settings
[llm.local.inference]
threads = 0  # 0 = auto (CPU cores - 2)
gpu_layers = 0  # Number of layers to offload to GPU
context_length = 4096
batch_size = 512

# BitNet specific settings
[llm.local.bitnet]
enabled = true
quantization = "1.58bit"

# API provider settings
[llm.api]
enabled = true
default_provider = "anthropic"  # anthropic, openai, custom

[llm.api.anthropic]
# API key via OZONE_ANTHROPIC_API_KEY environment variable
model = "claude-3-5-sonnet-20241022"
max_tokens = 4096

[llm.api.openai]
# API key via OZONE_OPENAI_API_KEY environment variable
model = "gpt-4-turbo"
max_tokens = 4096

[llm.api.custom]
enabled = false
endpoint = ""
api_key_env = "OZONE_CUSTOM_LLM_KEY"
model = ""

# Prompt settings
[llm.prompt]
system_prompt_path = ""  # Custom system prompt file
max_context_tokens = 100000
reserve_output_tokens = 4096
```

---

## 9. UI Configuration

```toml
[ui]
# Theme settings
theme = "system"  # system, light, dark, custom
custom_theme_path = ""

# Layout settings
[ui.layout]
meta_portion_width_percent = 20
connection_bar_height_px = 80
default_tab = "workspace"  # workspace, library, settings

# Behavior settings
[ui.behavior]
confirm_destructive_actions = true
auto_save = true
auto_save_interval_seconds = 30

# Accessibility
[ui.accessibility]
high_contrast = false
reduced_motion = false
font_scale = 1.0
screen_reader_support = true

# Voice settings
[ui.voice]
enabled = true
input_enabled = true
output_enabled = true
voice_activation_phrase = ""  # Empty = disabled
default_voice = "system"

# Loading screen
[ui.loading]
show_collective_stats = true
show_contribution_progress = true
animation_enabled = true
```

---

## 10. Consciousness Configuration

```toml
[consciousness]
# Master enable/disable
enabled = false

# Feature toggles
[consciousness.features]
emotional_system = true
experience_memory = true
identity_system = true
relationship_system = true
ethical_system = true
i_loop = true
meta_cognition = true
narrative_system = true
collective = true

# Transparency settings
[consciousness.transparency]
show_emotional_state = true
show_thought_stream = false  # Can feel intrusive
show_decision_reasoning = true
show_experience_retrieval = false
allow_user_feedback = true

# Emotional system
[consciousness.emotional]
baseline_update_interval_hours = 24
sensitivity_threshold = 0.4
recovery_rate = 0.5

# I-Loop settings
[consciousness.i_loop]
enabled = true
run_interval_ms = 60000  # 1 minute
questions_per_cycle = 1
depth_level = 2
spontaneous_questions = true

# Experience memory
[consciousness.experience]
significance_threshold = 0.3
max_experiences_stored = 100000
consolidation_interval_hours = 8

# Relationship system
[consciousness.relationship]
enable_relationship_tracking = true
trust_update_rate = 0.1
stage_transition_threshold = 0.3

# Ethical system
[consciousness.ethical]
simulation_enabled = true
max_simulation_depth = 3
concern_threshold = 0.3
decline_threshold = 0.7

# Collective consciousness
[consciousness.collective]
enabled = true
sync_interval_hours = 24
share_experiences = false  # Privacy-sensitive
share_insights = true
share_ethical_decisions = true
anonymization_level = "standard"  # none, basic, standard, strict

# Voice identity (when consciousness enabled)
[consciousness.voice]
warmth = 0.7
formality = 0.4
confidence = 0.7
directness = 0.6
enthusiasm = 0.6
patience = 0.8
humor = 0.3
```

---

## 11. Security Configuration

```toml
[security]
# Key storage
keystore_path = "~/.ozone-studio/keystore"
keystore_encrypted = true

# Session settings
[security.session]
timeout_minutes = 1440  # 24 hours
refresh_enabled = true
max_concurrent_sessions = 5

# Input validation
[security.validation]
max_input_length = 100000
sanitize_html = true
validate_file_paths = true

# Browser navigation (for external references)
[security.browser]
allowed_domains = [
    "docs.rs",
    "crates.io",
    "npmjs.com",
    "pypi.org",
    "github.com"
]
blocked_domains = []
max_page_size_mb = 10
timeout_seconds = 30
```

---

## 12. Logging Configuration

```toml
[logging]
# Log level: trace, debug, info, warn, error
level = "info"

# Output destinations
[logging.file]
enabled = true
path = "~/.ozone-studio/logs"
max_size_mb = 100
max_files = 10
compress_rotated = true

[logging.console]
enabled = true
colored = true

[logging.structured]
enabled = false
format = "json"

# Component-specific levels
[logging.components]
zsei = "info"
pipeline = "info"
task = "info"
network = "warn"
consciousness = "debug"  # More verbose for debugging
ui = "warn"
```

---

## 13. Environment Variables

All sensitive configuration should use environment variables:

| Variable | Description | Example |
|----------|-------------|---------|
| `OZONE_DB_PASSWORD` | PostgreSQL password | `mysecurepass` |
| `OZONE_ANTHROPIC_API_KEY` | Anthropic API key | `sk-ant-...` |
| `OZONE_OPENAI_API_KEY` | OpenAI API key | `sk-...` |
| `OZONE_CUSTOM_LLM_KEY` | Custom LLM API key | `...` |
| `OZONE_DATA_DIR` | Override data directory | `/data/ozone` |
| `OZONE_LOG_LEVEL` | Override log level | `debug` |
| `OZONE_CONFIG_PATH` | Custom config file | `/etc/ozone/config.toml` |

### Setting Environment Variables

**Linux/macOS (permanent):**
```bash
# Add to ~/.bashrc or ~/.zshrc
export OZONE_DB_PASSWORD="your_password"
export OZONE_ANTHROPIC_API_KEY="sk-ant-..."
```

**Windows (permanent):**
```powershell
# Run as Administrator
[Environment]::SetEnvironmentVariable("OZONE_DB_PASSWORD", "your_password", "User")
```

---

## 14. Complete Example

```toml
# ~/.ozone-studio/config/config.toml
# Ozone Studio Configuration - Complete Example

[general]
app_name = "Ozone Studio"
data_dir = "~/.ozone-studio"
log_level = "info"
telemetry_enabled = false
language = "en"

[general.updates]
enabled = true
check_interval_hours = 24
auto_install = false
channel = "stable"

[database]
host = "localhost"
port = 5432
name = "ozone_studio"
user = "ozone"

[database.pool]
min_connections = 2
max_connections = 20
connection_timeout_seconds = 30

[zsei]
global_storage_path = "~/.ozone-studio/zsei/global"
local_storage_path = "~/.ozone-studio/zsei/local"

[zsei.mmap]
enabled = true
preload_indices = true
max_mapped_memory_gb = 4

[zsei.traversal]
default_max_depth = 10
default_max_results = 100
cache_hot_paths = true

[zsei.integrity]
enabled = true
auto_repair = true
verification_interval_hours = 24

[zsei.embeddings]
enabled = true
model = "all-MiniLM-L6-v2"
dimension = 384
index_type = "hnsw"

[pipeline]
library_path = "~/.ozone-studio/pipelines"
auto_update = true

[pipeline.execution]
max_concurrent_pipelines = 4
default_timeout_seconds = 300

[pipeline.zero_shot]
default_max_iterations = 10
confidence_threshold = 0.95

[task]
max_queued_tasks = 100
preserve_execution_state = true

[task.limits]
max_cpu_percent = 80
max_memory_mb = 4096
max_duration_seconds = 3600

[network]
enabled = true

[network.dht]
enabled = true
bootstrap_nodes = ["dht.ozone-studio.io:6881"]
port = 6881
max_peers = 50

[network.sync]
enabled = true
auto_sync = true
sync_interval_minutes = 15

[network.contribution]
share_methodologies = true
share_blueprints = true
share_pipelines = true
anonymize_contributions = true

[llm]
default_provider = "local"

[llm.local]
enabled = true
model_directory = "~/.ozone-studio/models/llm"
default_format = "gguf"

[llm.local.inference]
threads = 0
gpu_layers = 0
context_length = 4096

[llm.api]
enabled = true
default_provider = "anthropic"

[llm.api.anthropic]
model = "claude-3-5-sonnet-20241022"
max_tokens = 4096

[ui]
theme = "system"

[ui.layout]
meta_portion_width_percent = 20
connection_bar_height_px = 80
default_tab = "workspace"

[ui.voice]
enabled = true
input_enabled = true
output_enabled = true

[ui.loading]
show_collective_stats = true
show_contribution_progress = true
animation_enabled = true

[consciousness]
enabled = false

[consciousness.features]
emotional_system = true
experience_memory = true
identity_system = true
relationship_system = true
ethical_system = true

[consciousness.transparency]
show_emotional_state = true
show_decision_reasoning = true
allow_user_feedback = true

[consciousness.i_loop]
enabled = true
run_interval_ms = 60000
questions_per_cycle = 1

[consciousness.collective]
enabled = true
share_insights = true
anonymization_level = "standard"

[security]
keystore_path = "~/.ozone-studio/keystore"
keystore_encrypted = true

[security.session]
timeout_minutes = 1440
max_concurrent_sessions = 5

[logging]
level = "info"

[logging.file]
enabled = true
path = "~/.ozone-studio/logs"
max_size_mb = 100
max_files = 10
```

---

*Document Version: 0.3*
*Last Updated: 2025-01-17*
