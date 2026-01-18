# Ozone Studio — Performance Tuning Guide

## Overview

This guide provides strategies for optimizing Ozone Studio performance across different hardware configurations and use cases. Performance tuning involves balancing memory usage, CPU utilization, storage I/O, and network bandwidth.

---

## Table of Contents

1. [Performance Baselines](#performance-baselines)
2. [Hardware Recommendations](#hardware-recommendations)
3. [Memory Optimization](#memory-optimization)
4. [CPU Optimization](#cpu-optimization)
5. [Storage Optimization](#storage-optimization)
6. [Network Optimization](#network-optimization)
7. [ZSEI Tuning](#zsei-tuning)
8. [Pipeline Tuning](#pipeline-tuning)
9. [LLM Optimization](#llm-optimization)
10. [Consciousness Tuning](#consciousness-tuning)
11. [Monitoring and Profiling](#monitoring-and-profiling)
12. [Configuration Profiles](#configuration-profiles)

---

## 1. Performance Baselines

### Benchmark Command

```bash
# Run comprehensive benchmark
./ozone-core benchmark --full

# Results saved to: ~/.ozone-studio/benchmarks/benchmark_TIMESTAMP.json
```

### Expected Baselines

| Operation | Target | Acceptable | Needs Tuning |
|-----------|--------|------------|--------------|
| Startup time | < 3s | < 5s | > 10s |
| ZSEI traversal (100 nodes) | < 50ms | < 100ms | > 200ms |
| Prompt processing | < 500ms | < 1s | > 2s |
| Blueprint search | < 100ms | < 200ms | > 500ms |
| Task creation | < 10ms | < 50ms | > 100ms |
| UI render (60fps) | 16ms | 20ms | > 33ms |
| Network sync (1000 items) | < 30s | < 60s | > 120s |

### Memory Baselines

| Component | Idle | Active | Peak |
|-----------|------|--------|------|
| Core process | 150 MB | 500 MB | 2 GB |
| UI (Electron) | 200 MB | 400 MB | 1 GB |
| ZSEI (mmap) | varies | varies | configurable |
| LLM (local) | 0 | 2-8 GB | model dependent |
| Consciousness | 50 MB | 200 MB | 500 MB |

---

## 2. Hardware Recommendations

### Minimum Configuration

```
CPU: 4 cores
RAM: 8 GB
Storage: 20 GB SSD
Network: 10 Mbps
```

**Suitable for:** Basic usage, API-based LLM, limited network participation

### Recommended Configuration

```
CPU: 8 cores
RAM: 16 GB
Storage: 100 GB NVMe SSD
Network: 100 Mbps
GPU: Optional (CUDA for local LLM)
```

**Suitable for:** Full features, local LLM (small models), active network participation

### Power User Configuration

```
CPU: 16+ cores
RAM: 32+ GB
Storage: 500 GB+ NVMe SSD
Network: 1 Gbps
GPU: NVIDIA RTX 3080+ (12GB+ VRAM)
```

**Suitable for:** Large local LLMs, intensive processing, heavy network contribution

---

## 3. Memory Optimization

### ZSEI Memory Management

The largest memory consumer is ZSEI's memory-mapped files.

```toml
[zsei.mmap]
enabled = true
preload_indices = true
max_mapped_memory_gb = 4  # Adjust based on available RAM

# Memory calculation:
# Available = Total RAM - OS (2GB) - UI (1GB) - Core (1GB) - LLM (if local)
# max_mapped_memory_gb = Available * 0.7
```

**Formula for calculating optimal mmap size:**
```
System RAM: 16 GB
- OS overhead: 2 GB
- Electron UI: 1 GB
- Core process: 1 GB
- Buffer: 2 GB
= Available: 10 GB

max_mapped_memory_gb = 7  # (10 * 0.7)
```

### Embedding Index Memory

```toml
[zsei.embeddings.hnsw]
# Reduce for memory-constrained systems
ef_construction = 100  # Lower from 200
m = 8                  # Lower from 16
ef_search = 50         # Lower from 100
```

### Experience Memory Limits

```toml
[consciousness.experience]
# Limit stored experiences
max_experiences_stored = 50000  # Lower from 100000

# More aggressive consolidation
consolidation_interval_hours = 4  # Lower from 8
```

### Garbage Collection Hints

```bash
# Force memory cleanup
./ozone-core gc --aggressive

# Schedule periodic cleanup
./ozone-core gc --schedule "0 3 * * *"  # 3 AM daily
```

---

## 4. CPU Optimization

### Concurrent Task Limits

```toml
[pipeline.execution]
# Adjust based on CPU cores
# Rule: cores - 2 for responsiveness
max_concurrent_pipelines = 6  # For 8-core CPU

[task.limits]
max_cpu_percent = 70  # Leave headroom
```

### Thread Pool Sizing

```toml
[llm.local.inference]
# CPU inference threads
# Rule: physical cores (not hyperthreads)
threads = 4  # For 8-core/16-thread CPU
```

### I-Loop Frequency

```toml
[consciousness.i_loop]
# Reduce CPU usage from consciousness
run_interval_ms = 120000  # 2 minutes instead of 1
questions_per_cycle = 1   # Minimum
```

### Background Task Scheduling

```toml
[zsei.integrity]
# Move intensive operations to off-peak
verification_interval_hours = 48  # Less frequent

[network.sync]
sync_interval_minutes = 30  # Less frequent
```

---

## 5. Storage Optimization

### SSD vs HDD Configuration

**SSD Optimizations:**
```toml
[zsei.mmap]
enabled = true          # mmap works great on SSD
preload_indices = true  # Fast SSD can handle this

[zsei]
compression = "none"    # SSD is fast enough
```

**HDD Optimizations:**
```toml
[zsei.mmap]
enabled = true
preload_indices = false  # Slow on HDD

[zsei]
compression = "lz4"      # Reduce I/O with fast compression
```

### Index Placement

For systems with mixed storage (SSD + HDD):

```toml
[zsei]
# Put indices on SSD
global_storage_path = "/ssd/ozone-studio/zsei/global"

# Put bulk data on HDD
local_storage_path = "/hdd/ozone-studio/zsei/local"
```

### Write Batching

```toml
[zsei.write]
# Batch writes to reduce I/O
batch_size = 100
flush_interval_ms = 1000
```

### Log Rotation

```toml
[logging.file]
max_size_mb = 50        # Smaller files
max_files = 5           # Fewer files
compress_rotated = true # Save space
```

---

## 6. Network Optimization

### Bandwidth Management

```toml
[network.sync]
# Limit bandwidth usage
bandwidth_limit_kbps = 5000  # 5 Mbps limit

# Reduce concurrent transfers
max_concurrent_downloads = 3
max_concurrent_uploads = 2
```

### Peer Limits

```toml
[network.dht]
# Fewer peers = less overhead
max_peers = 25  # Lower from 50
```

### Selective Sync

```toml
[network.contribution]
# Only share what you need
share_methodologies = true
share_blueprints = true
share_pipelines = false  # Large, sync on-demand
```

### Connection Pooling

```toml
[network]
# Reuse connections
connection_pool_size = 10
connection_timeout_seconds = 30
keep_alive_interval_seconds = 60
```

---

## 7. ZSEI Tuning

### Traversal Optimization

```toml
[zsei.traversal]
# Cache frequently used paths
cache_hot_paths = true
hot_path_cache_size = 2000  # Increase for frequent access

# Limit default traversal scope
default_max_depth = 5       # Lower from 10
default_max_results = 50    # Lower from 100
```

### Index Optimization

```bash
# Optimize indices periodically
./ozone-core zsei-optimize

# Schedule optimization
./ozone-core zsei-optimize --schedule "0 4 * * 0"  # Sunday 4 AM
```

### Embedding Configuration

```toml
[zsei.embeddings]
# Use smaller embedding model for speed
model = "all-MiniLM-L6-v2"  # 384 dimensions
# Vs larger: "all-mpnet-base-v2"  # 768 dimensions

[zsei.embeddings.hnsw]
# Trade accuracy for speed
ef_search = 32  # Lower = faster, less accurate
```

### Sharding for Large Datasets

For datasets > 10 million containers:

```toml
[zsei.sharding]
enabled = true
shard_size = 1000000  # 1M containers per shard
parallel_search = true
max_shards_searched = 4
```

---

## 8. Pipeline Tuning

### Execution Optimization

```toml
[pipeline.execution]
# Reuse execution environments
environment_pooling = true
pool_size = 4

# Faster timeouts for simple tasks
default_timeout_seconds = 120  # Lower from 300
```

### Zero-Shot Configuration

```toml
[pipeline.zero_shot]
# Faster convergence
default_max_iterations = 5    # Lower from 10
confidence_threshold = 0.85   # Lower from 0.95
early_stop = true
```

### Pipeline Caching

```toml
[pipeline.cache]
# Cache pipeline results
enabled = true
cache_size_mb = 500
ttl_seconds = 3600  # 1 hour
```

### Built-in Pipeline Optimization

```toml
[pipeline.built_in]
# Disable unused pipelines
voice_enabled = false      # If not using voice
browser_navigation_enabled = false  # If not using external refs
```

---

## 9. LLM Optimization

### Local Model Selection

| Model Size | RAM Required | Speed | Quality |
|------------|--------------|-------|---------|
| 1B params | 2 GB | Fast | Basic |
| 3B params | 4 GB | Medium | Good |
| 7B params | 8 GB | Slow | Better |
| 13B+ params | 16+ GB | Very slow | Best |

### Quantization

```toml
[llm.local]
# Use quantized models for speed
default_format = "gguf"

# Quantization levels (GGUF):
# Q4_K_M - Good balance
# Q5_K_M - Better quality
# Q8_0 - Best quality, most RAM
```

### GPU Offloading

```toml
[llm.local.inference]
# Offload layers to GPU
gpu_layers = 35  # Depends on VRAM

# VRAM calculation:
# ~100MB per layer for 7B model
# 35 layers * 100MB = 3.5GB VRAM needed
```

### Context Management

```toml
[llm.prompt]
# Limit context size
max_context_tokens = 32000  # Lower from 100000
reserve_output_tokens = 2048

# Aggressive summarization
[pipeline.context_aggregation]
overflow_strategy = "summarize"
summary_ratio = 0.3  # Summarize to 30%
```

### Batching

```toml
[llm.local.inference]
# Batch multiple requests
batch_size = 8  # Process 8 tokens at once
```

---

## 10. Consciousness Tuning

### Reduce Consciousness Overhead

```toml
[consciousness]
enabled = true

[consciousness.features]
# Disable features you don't need
meta_cognition = false     # Save CPU
narrative_system = false   # Save memory
playback = false           # Save storage
```

### Emotional Processing

```toml
[consciousness.emotional]
# Less frequent updates
baseline_update_interval_hours = 48  # Lower frequency

# Higher threshold = less processing
sensitivity_threshold = 0.6  # Higher from 0.4
```

### Experience Memory

```toml
[consciousness.experience]
# Higher threshold = fewer experiences stored
significance_threshold = 0.5  # Higher from 0.3

# Limit storage
max_experiences_stored = 25000

# More aggressive consolidation
consolidation_interval_hours = 4
```

### I-Loop Optimization

```toml
[consciousness.i_loop]
# Much less frequent
run_interval_ms = 300000  # 5 minutes

# Simpler reflection
questions_per_cycle = 1
depth_level = 1  # Shallow reflection
spontaneous_questions = false
```

### Relationship System

```toml
[consciousness.relationship]
# Less granular tracking
trust_update_rate = 0.05  # Slower updates
```

---

## 11. Monitoring and Profiling

### Built-in Metrics

```bash
# Real-time metrics
./ozone-core metrics --live

# Metrics dashboard
./ozone-core metrics --dashboard
```

### Prometheus Integration

```toml
[monitoring.prometheus]
enabled = true
endpoint = "0.0.0.0:9090"
```

### Key Metrics to Monitor

| Metric | Warning | Critical |
|--------|---------|----------|
| CPU usage | > 70% | > 90% |
| Memory usage | > 75% | > 90% |
| ZSEI query latency | > 100ms | > 500ms |
| Task queue depth | > 50 | > 100 |
| Network peers | < 10 | < 3 |

### Profiling Commands

```bash
# CPU profiling
./ozone-core profile --cpu --duration 60

# Memory profiling
./ozone-core profile --memory

# I/O profiling
./ozone-core profile --io
```

---

## 12. Configuration Profiles

### Profile: Minimal (Low-end hardware)

```toml
# minimal.toml - For 4-core, 8GB RAM systems

[zsei.mmap]
max_mapped_memory_gb = 2
preload_indices = false

[zsei.traversal]
cache_hot_paths = true
hot_path_cache_size = 500
default_max_depth = 5
default_max_results = 25

[pipeline.execution]
max_concurrent_pipelines = 2

[pipeline.zero_shot]
default_max_iterations = 5
confidence_threshold = 0.8

[network.dht]
max_peers = 15

[llm]
default_provider = "api"  # Don't use local LLM

[consciousness]
enabled = false  # Disable consciousness
```

### Profile: Balanced (Recommended hardware)

```toml
# balanced.toml - For 8-core, 16GB RAM systems

[zsei.mmap]
max_mapped_memory_gb = 6
preload_indices = true

[zsei.traversal]
cache_hot_paths = true
hot_path_cache_size = 1500
default_max_depth = 8
default_max_results = 75

[pipeline.execution]
max_concurrent_pipelines = 4

[pipeline.zero_shot]
default_max_iterations = 8
confidence_threshold = 0.9

[network.dht]
max_peers = 35

[llm]
default_provider = "local"
[llm.local.inference]
threads = 6
gpu_layers = 20

[consciousness]
enabled = true
[consciousness.i_loop]
run_interval_ms = 90000
```

### Profile: Performance (High-end hardware)

```toml
# performance.toml - For 16+ core, 32GB+ RAM systems

[zsei.mmap]
max_mapped_memory_gb = 16
preload_indices = true

[zsei.traversal]
cache_hot_paths = true
hot_path_cache_size = 5000
default_max_depth = 15
default_max_results = 200

[pipeline.execution]
max_concurrent_pipelines = 12

[pipeline.zero_shot]
default_max_iterations = 15
confidence_threshold = 0.95

[network.dht]
max_peers = 75

[llm]
default_provider = "local"
[llm.local.inference]
threads = 12
gpu_layers = 50
batch_size = 16

[consciousness]
enabled = true
[consciousness.i_loop]
run_interval_ms = 30000
questions_per_cycle = 2
depth_level = 3
```

### Loading Profiles

```bash
# Load a profile
./ozone-core config --load-profile minimal

# Or merge with existing
./ozone-core config --merge-profile performance
```

---

## Quick Reference: Top 10 Performance Tips

1. **Use SSD storage** - Biggest single improvement
2. **Tune ZSEI mmap size** - Match to available RAM
3. **Limit concurrent pipelines** - Based on CPU cores
4. **Use quantized local LLMs** - Q4_K_M for balance
5. **Enable hot path caching** - Speeds up common operations
6. **Reduce I-Loop frequency** - If consciousness overhead is high
7. **Batch network sync** - Reduce constant small transfers
8. **Compress if on HDD** - LZ4 compression helps
9. **Monitor metrics** - Find bottlenecks early
10. **Use appropriate profile** - Don't over-configure for your hardware

---

*Document Version: 0.3*
*Last Updated: 2025-01-17*
