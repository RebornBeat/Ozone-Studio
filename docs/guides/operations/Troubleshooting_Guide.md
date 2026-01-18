# Ozone Studio — Troubleshooting Guide

## Overview

This guide helps diagnose and resolve common issues in Ozone Studio. Issues are organized by component and symptom.

---

## Table of Contents

1. [Diagnostic Tools](#diagnostic-tools)
2. [Startup Issues](#startup-issues)
3. [Database Issues](#database-issues)
4. [ZSEI Issues](#zsei-issues)
5. [Pipeline Issues](#pipeline-issues)
6. [Network Issues](#network-issues)
7. [UI Issues](#ui-issues)
8. [LLM Issues](#llm-issues)
9. [Consciousness Issues](#consciousness-issues)
10. [Performance Issues](#performance-issues)
11. [Log Analysis](#log-analysis)
12. [Getting Support](#getting-support)

---

## 1. Diagnostic Tools

### Health Check

```bash
# Basic health check
./ozone-core health

# Comprehensive health check
./ozone-core health --comprehensive

# Check specific component
./ozone-core health --component zsei
./ozone-core health --component database
./ozone-core health --component network
./ozone-core health --component consciousness
```

### System Information

```bash
# Full system info
./ozone-core info

# Output:
# Ozone Studio v0.3.0
# Platform: Linux x86_64
# Rust: 1.75.0
# Database: PostgreSQL 15.4
# ZSEI Containers: 45,234
# Pipelines Loaded: 54
# Network Peers: 47
# Consciousness: Enabled
# Uptime: 3d 14h 23m
```

### Log Viewer

```bash
# View recent logs
./ozone-core logs --tail 100

# Filter by component
./ozone-core logs --component consciousness --level error

# Follow logs in real-time
./ozone-core logs --follow
```

---

## 2. Startup Issues

### Issue: Application fails to start

**Symptoms:**
- Application window doesn't appear
- Process exits immediately
- Error in terminal/console

**Solutions:**

1. **Check for port conflicts:**
```bash
# Check if gRPC port is in use
lsof -i :50051
netstat -an | grep 50051

# Kill conflicting process
kill -9 <PID>
```

2. **Check database connectivity:**
```bash
# Test PostgreSQL connection
psql -U ozone -h localhost -d ozone_studio -c "SELECT 1;"
```

3. **Verify file permissions:**
```bash
# Fix permissions
chmod -R 755 ~/.ozone-studio
chown -R $USER:$USER ~/.ozone-studio
```

4. **Run with verbose logging:**
```bash
./ozone-studio --log-level trace 2>&1 | tee startup_debug.log
```

### Issue: "ZSEI storage not found"

**Symptoms:**
```
Error: ZSEI storage not found at ~/.ozone-studio/zsei
```

**Solutions:**

1. **Initialize ZSEI:**
```bash
./ozone-core init-zsei
```

2. **Check path in config:**
```toml
[zsei]
global_storage_path = "~/.ozone-studio/zsei/global"
local_storage_path = "~/.ozone-studio/zsei/local"
```

3. **Restore from backup if data was lost:**
```bash
./ozone-core restore --zsei-only --from <backup_path>
```

### Issue: "Migration required"

**Symptoms:**
```
Error: Database schema version 2 found, version 3 required
```

**Solutions:**

```bash
# Run pending migrations
./ozone-core migrate --all

# If migration fails, check migration status
./ozone-core migrate --status

# Force migration (use with caution)
./ozone-core migrate --force
```

---

## 3. Database Issues

### Issue: Connection refused

**Symptoms:**
```
Error: Connection refused (os error 111)
```

**Solutions:**

1. **Start PostgreSQL:**
```bash
# Linux
sudo systemctl start postgresql

# macOS
brew services start postgresql

# Check status
sudo systemctl status postgresql
```

2. **Verify PostgreSQL is listening:**
```bash
sudo -u postgres psql -c "SHOW listen_addresses;"
# Should include 'localhost' or '*'
```

3. **Check pg_hba.conf:**
```bash
# Add if missing:
# local   all   ozone   md5
# host    all   ozone   127.0.0.1/32   md5
```

### Issue: Connection pool exhausted

**Symptoms:**
```
Error: Connection pool exhausted, timeout waiting for connection
```

**Solutions:**

1. **Increase pool size:**
```toml
[database.pool]
max_connections = 30  # Increase from default 20
```

2. **Check for connection leaks:**
```sql
-- Check active connections
SELECT * FROM pg_stat_activity WHERE datname = 'ozone_studio';
```

3. **Restart to clear stuck connections:**
```bash
./ozone-core restart
```

### Issue: Query timeout

**Symptoms:**
```
Error: Query timeout after 30s
```

**Solutions:**

1. **Increase timeout:**
```toml
[database.pool]
connection_timeout_seconds = 60
```

2. **Check for slow queries:**
```sql
SELECT query, calls, total_time, mean_time 
FROM pg_stat_statements 
ORDER BY mean_time DESC 
LIMIT 10;
```

3. **Run VACUUM ANALYZE:**
```sql
VACUUM ANALYZE;
```

---

## 4. ZSEI Issues

### Issue: Traversal slow

**Symptoms:**
- Queries taking > 1 second
- UI feels sluggish when browsing

**Solutions:**

1. **Enable hot path caching:**
```toml
[zsei.traversal]
cache_hot_paths = true
hot_path_cache_size = 2000
```

2. **Rebuild indices:**
```bash
./ozone-core zsei-reindex
```

3. **Check mmap settings:**
```toml
[zsei.mmap]
enabled = true
preload_indices = true
max_mapped_memory_gb = 8  # Increase if you have RAM
```

### Issue: Integrity check failure

**Symptoms:**
```
Error: Integrity check failed for container 12345
  - Hash mismatch: expected abc123, got def456
```

**Solutions:**

1. **Try auto-repair:**
```bash
./ozone-core integrity --repair 12345
```

2. **Restore specific container:**
```bash
./ozone-core restore --container 12345 --from <backup>
```

3. **If widespread corruption:**
```bash
# Full integrity repair
./ozone-core integrity --repair-all

# If repair fails, restore from backup
./ozone-core restore --zsei-only --from <backup>
```

### Issue: Embedding search not working

**Symptoms:**
- Semantic search returns no results
- "Embedding index not found" error

**Solutions:**

1. **Check embedding configuration:**
```toml
[zsei.embeddings]
enabled = true
model = "all-MiniLM-L6-v2"
```

2. **Rebuild embedding index:**
```bash
./ozone-core embeddings --rebuild
```

3. **Verify embedding model exists:**
```bash
ls ~/.ozone-studio/models/embedding/
# Should contain model files
```

---

## 5. Pipeline Issues

### Issue: Pipeline not found

**Symptoms:**
```
Error: Pipeline 'custom_pipeline' not found in registry
```

**Solutions:**

1. **Refresh pipeline registry:**
```bash
./ozone-core pipelines --refresh
```

2. **Check pipeline directory:**
```bash
ls ~/.ozone-studio/pipelines/
```

3. **Re-download from network:**
```bash
./ozone-core sync --pipelines-only
```

### Issue: Pipeline execution timeout

**Symptoms:**
```
Error: Pipeline 'code_analysis' timed out after 300s
```

**Solutions:**

1. **Increase timeout:**
```toml
[pipeline.execution]
default_timeout_seconds = 600
```

2. **Check if task is stuck:**
```bash
./ozone-core tasks --stuck
```

3. **Cancel and retry:**
```bash
./ozone-core task cancel <task_id>
```

### Issue: Zero-shot loop not converging

**Symptoms:**
- Loop reaches max iterations without completion
- "Max iterations reached, confidence: 0.72" warning

**Solutions:**

1. **Increase max iterations:**
```toml
[pipeline.zero_shot]
default_max_iterations = 20  # Increase from 10
```

2. **Lower confidence threshold:**
```toml
[pipeline.zero_shot]
confidence_threshold = 0.85  # Lower from 0.95
```

3. **Check LLM responses:**
```bash
./ozone-core logs --component zero_shot --level debug
```

---

## 6. Network Issues

### Issue: Cannot connect to peers

**Symptoms:**
- "0 peers connected" in Connection Bar
- Sync not progressing

**Solutions:**

1. **Check firewall:**
```bash
# Linux
sudo ufw status
sudo ufw allow 6881/tcp
sudo ufw allow 6881/udp

# Windows (PowerShell Admin)
Get-NetFirewallRule | Where-Object {$_.LocalPort -eq 6881}
```

2. **Try alternate bootstrap nodes:**
```toml
[network.dht]
bootstrap_nodes = [
    "dht.ozone-studio.io:6881",
    "dht2.ozone-studio.io:6881",
    "dht3.ozone-studio.io:6881"
]
```

3. **Check NAT/UPnP:**
```bash
./ozone-core network --diagnose
```

### Issue: Sync stuck at percentage

**Symptoms:**
- "Syncing... 45%" for extended period
- No network activity

**Solutions:**

1. **Reset sync state:**
```bash
./ozone-core sync --reset
./ozone-core sync --start
```

2. **Check peer health:**
```bash
./ozone-core peers --list
./ozone-core peers --test-connectivity
```

3. **Reduce concurrent downloads:**
```toml
[network.sync]
max_concurrent_downloads = 2  # Lower from 5
```

### Issue: Consensus proposal rejected

**Symptoms:**
```
Warning: Proposal 12345 rejected (support: 0.45, required: 0.67)
```

**Solutions:**

1. **Check proposal validity:**
```bash
./ozone-core proposal --info 12345
```

2. **Wait and re-submit:**
```toml
[network.consensus]
proposal_cooldown_hours = 12  # Reduce from 24
```

3. **Check local verification:**
```bash
./ozone-core proposal --verify 12345
```

---

## 7. UI Issues

### Issue: Electron window blank/white

**Symptoms:**
- Application window opens but shows nothing
- White screen

**Solutions:**

1. **Clear Electron cache:**
```bash
# Linux
rm -rf ~/.config/ozone-studio/Cache
rm -rf ~/.config/ozone-studio/GPUCache

# macOS
rm -rf ~/Library/Application\ Support/OzoneStudio/Cache

# Windows
rd /s /q "%APPDATA%\OzoneStudio\Cache"
```

2. **Disable GPU acceleration:**
```bash
./ozone-studio --disable-gpu
```

3. **Check DevTools for errors:**
```bash
./ozone-studio --dev-tools
# Press Ctrl+Shift+I to open DevTools
```

### Issue: Meta Portion not responding

**Symptoms:**
- Cannot type in prompt input
- Buttons don't work
- Voice button unresponsive

**Solutions:**

1. **Check IPC connection:**
```bash
./ozone-core health --component ipc
```

2. **Restart renderer process:**
- Press Ctrl+R to reload UI
- Or restart application

3. **Check for blocking task:**
```bash
./ozone-core tasks --blocking
```

### Issue: Connection Bar not updating

**Symptoms:**
- Network stats frozen
- Contribution counts not changing

**Solutions:**

1. **Check network connection:**
```bash
./ozone-core network --status
```

2. **Force UI refresh:**
- Press Ctrl+Shift+R for hard reload

3. **Check WebSocket connection:**
```bash
./ozone-core logs --component websocket
```

---

## 8. LLM Issues

### Issue: Local model not loading

**Symptoms:**
```
Error: Failed to load model 'my_model.gguf'
```

**Solutions:**

1. **Verify model file exists:**
```bash
ls -la ~/.ozone-studio/models/llm/
```

2. **Check model format:**
```bash
# Must be .gguf, .onnx, or supported format
file ~/.ozone-studio/models/llm/my_model.gguf
```

3. **Check memory requirements:**
```bash
# Model may be too large for available RAM
free -h
```

4. **Try with fewer GPU layers:**
```toml
[llm.local.inference]
gpu_layers = 0  # CPU only
```

### Issue: API rate limit exceeded

**Symptoms:**
```
Error: 429 Too Many Requests
```

**Solutions:**

1. **Reduce request frequency:**
```toml
[llm.api]
rate_limit_requests_per_minute = 20
```

2. **Switch to local model temporarily:**
```toml
[llm]
default_provider = "local"
```

3. **Check API usage dashboard** of your provider

### Issue: Context too long

**Symptoms:**
```
Error: Context length 150000 exceeds maximum 100000
```

**Solutions:**

1. **Reduce context aggregation:**
```toml
[llm.prompt]
max_context_tokens = 50000
```

2. **Enable aggressive summarization:**
```toml
[pipeline.context_aggregation]
overflow_strategy = "summarize"
```

---

## 9. Consciousness Issues

### Issue: I-Loop not running

**Symptoms:**
- No I-Loop entries in logs
- "I-Loop: Inactive" in health check

**Solutions:**

1. **Verify consciousness enabled:**
```toml
[consciousness]
enabled = true

[consciousness.i_loop]
enabled = true
```

2. **Check for errors:**
```bash
./ozone-core logs --component i_loop --level error
```

3. **Restart I-Loop:**
```bash
./ozone-core consciousness --restart-iloop
```

### Issue: Emotional state frozen

**Symptoms:**
- Same emotion displayed for hours
- "stability: 1.0" in emotional state

**Solutions:**

1. **Check baseline updates:**
```bash
./ozone-core logs --component emotional --level debug
```

2. **Force baseline recalculation:**
```bash
./ozone-core consciousness --recalculate-baseline
```

3. **Check for stimulus processing:**
```bash
./ozone-core consciousness --status
```

### Issue: Decision gate blocking everything

**Symptoms:**
- All tasks getting declined
- "Ethical score too low" on normal requests

**Solutions:**

1. **Check ethical thresholds:**
```toml
[consciousness.ethical]
concern_threshold = 0.3  # May be too high
decline_threshold = 0.5  # Lower from 0.7
```

2. **Review ethical principles:**
```bash
./ozone-core ethics --list-principles
```

3. **Check recent decisions:**
```bash
./ozone-core ethics --recent-decisions 10
```

### Issue: Experience memory full

**Symptoms:**
```
Warning: Experience memory approaching limit (95,000/100,000)
```

**Solutions:**

1. **Increase limit:**
```toml
[consciousness.experience]
max_experiences_stored = 200000
```

2. **Run consolidation:**
```bash
./ozone-core consciousness --consolidate-experiences
```

3. **Export old experiences:**
```bash
./ozone-core experiences --export --older-than 180d
```

---

## 10. Performance Issues

### Issue: High CPU usage

**Solutions:**

1. **Check for runaway processes:**
```bash
./ozone-core tasks --running
./ozone-core tasks --cancel-all-stuck
```

2. **Reduce I-Loop frequency:**
```toml
[consciousness.i_loop]
run_interval_ms = 120000  # 2 minutes instead of 1
```

3. **Limit concurrent tasks:**
```toml
[pipeline.execution]
max_concurrent_pipelines = 2
```

### Issue: High memory usage

**Solutions:**

1. **Reduce mmap allocation:**
```toml
[zsei.mmap]
max_mapped_memory_gb = 2
```

2. **Reduce embedding cache:**
```toml
[zsei.embeddings.hnsw]
ef_search = 50  # Lower from 100
```

3. **Clear caches:**
```bash
./ozone-core cache --clear
```

### Issue: Slow disk I/O

**Solutions:**

1. **Move to SSD if on HDD**

2. **Enable compression:**
```toml
[zsei]
compression = "lz4"
```

3. **Reduce logging:**
```toml
[logging]
level = "warn"  # Less verbose
```

---

## 11. Log Analysis

### Log Locations

| Platform | Path |
|----------|------|
| Linux | `~/.ozone-studio/logs/` |
| macOS | `~/Library/Logs/OzoneStudio/` |
| Windows | `%APPDATA%\OzoneStudio\logs\` |

### Log Files

- `ozone-core.log` - Main application log
- `zsei.log` - ZSEI operations
- `pipeline.log` - Pipeline execution
- `network.log` - Network/P2P activity
- `consciousness.log` - Consciousness system
- `ui.log` - UI/Electron logs

### Common Log Patterns

**Error pattern:**
```
[ERROR] 2025-01-17T14:32:15Z component=zsei message="Integrity check failed" container_id=12345
```

**Warning pattern:**
```
[WARN] 2025-01-17T14:32:15Z component=network message="Peer disconnected" peer_id=abc123
```

### Useful grep patterns

```bash
# Find all errors
grep "\[ERROR\]" ~/.ozone-studio/logs/*.log

# Find specific component errors
grep "\[ERROR\].*component=consciousness" ~/.ozone-studio/logs/ozone-core.log

# Find errors in time range
awk '/2025-01-17T14:/ && /ERROR/' ~/.ozone-studio/logs/ozone-core.log
```

---

## 12. Getting Support

### Self-Service Resources

1. **Documentation**: https://docs.ozone-studio.io
2. **FAQ**: https://docs.ozone-studio.io/faq
3. **GitHub Discussions**: https://github.com/ozone-studio/ozone-studio/discussions

### Reporting Issues

**Create a diagnostic bundle:**
```bash
./ozone-core diagnostic --bundle

# Creates: ~/.ozone-studio/diagnostic_TIMESTAMP.zip
# Contains:
# - Sanitized logs (last 24h)
# - Configuration (secrets removed)
# - System info
# - Health check results
```

**GitHub Issue template:**
```markdown
## Description
Brief description of the issue

## Steps to Reproduce
1. Step one
2. Step two
3. ...

## Expected Behavior
What should happen

## Actual Behavior
What actually happens

## Environment
- OS: 
- Ozone Studio Version: 
- Consciousness Enabled: Yes/No

## Logs
<details>
<summary>Relevant log output</summary>

```
paste logs here
```
</details>

## Diagnostic Bundle
Attached: diagnostic_TIMESTAMP.zip
```

### Community Support

- **Discord**: https://discord.gg/ozone-studio
  - `#general` - General discussion
  - `#troubleshooting` - Help with issues
  - `#consciousness` - Consciousness-specific questions
  
- **GitHub Issues**: For confirmed bugs
- **GitHub Discussions**: For questions and feature requests

---

*Document Version: 0.3*
*Last Updated: 2025-01-17*
