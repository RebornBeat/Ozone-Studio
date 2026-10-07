# Runtime Memory Plan — pipelines, local models, shared state

Status: DRAFT for ZCode + CC review. Nothing here is implemented except the items marked DONE or IN PROGRESS in §4. No host run since the current edits, so no RAM numbers are measured yet.

## 1. What the code does today

- **Loops run in the host process.** Meta, I-loop, AMT (main and reexpansion), assistant ticker, ripple actors, task-queue workers and graph loops are all `tokio::spawn` tasks in one process (`src/lib.rs:1027-1102`, `src/orchestrator/actors.rs:452`, `src/task/mod.rs:1066,1896`, `src/orchestrator/amt_loop.rs:102,1646`, `src/orchestrator/i_loop.rs:65`, `src/orchestrator/meta_loop.rs:55`, `src/consciousness/assistant.rs:528`). They add no process RAM; they trigger the calls below.
- **Every pipeline call is a new OS process,** freed on exit (`src/pipeline/executor.rs:445-477`). Rust binaries run directly, `.py` under `python3`, `.js` under `node`. The host waits via `spawn_blocking` + `cmd.output()`, which buffers the child's full stdout in memory.
- **Concurrency cap rejects, does not queue.** `max_concurrent_pipelines = 10` (`src/config/mod.rs:380`); the 11th call returns an error (`src/pipeline/executor.rs:112-122`).
- **AMT lanes fan out in parallel.** `JoinSet` at `src/orchestrator/amt.rs:2347`, retry pass at `:2403`. Each lane is a prompt-pipeline (id 9) call, so N lanes means N concurrent children. Typical N is not verified.
- **Prompt pipeline model backends** (`assets/pipelines/general/prompt/main.rs`):
  - API (Anthropic, OpenAI): HTTP only, no local model.
  - GGUF: spawns `llama-cli` per call (`:529-558`). No server path.
  - BitNet: if `OZONE_LLAMA_SERVER_URL` is set and healthy, uses a persistent `llama-server` over HTTP (`:800-867`, returns real `usage.completion_tokens`). Otherwise spawns `llama-cli` per call (`:869+`). Opt-in only.
  - ONNX: spawns a Python bridge per call (`:613+`).
- **MCP tools are external, resident processes.** `src/mcp.rs` spawns nothing. The host calls their endpoints; each tool holds its RAM for as long as it runs.

## 2. The problem

Peak RAM is roughly: concurrent pipeline children × their RSS, plus a full model load per local call, plus resident tools. The cap rejects instead of queuing, so loops can crowd out user work and drop lane results. Model weights are loaded per call for GGUF always, and for BitNet unless the server is configured.

## 3. Target design

1. **Light pipelines** (Rust, no model): keep spawn-per-call. Memory frees on exit, so the only cost is peak concurrency.
2. **Heavy-init pipelines** (model load or Python ML runtime): a small pool of persistent workers per pipeline id, JSON lines over stdin/stdout. Pool size 1–2. Recycle after K calls or T idle minutes. Each loads its model once.
3. **One admission gate for every pipeline call.** A semaphore sized to the RAM budget replaces the reject at 10. Calls wait with a timeout instead of failing. User-initiated work has priority over loops. AMT lane fan-out acquires through the same gate.
4. **One resident local model server.** BitNet and GGUF both use a single `llama-server` per local model. Enabled by default when a local model is configured; GGUF gets a server path it doesn't have today. ONNX moves to a persistent Python worker.
5. **Warm-up.** Start the local model server at host boot only if a local model is configured. Everything else is lazy. Heavy workers unload after an idle timeout to return RAM.
6. **Visibility.** Monitor shows real child RSS read from `/proc` per pool and worker. No estimates.
7. **Accounting.** Token counts stay real: server `usage` fields and CLI perf output only, never word counts.

**Peak RAM target:** host + (gate × light-pipeline RSS) + (heavy-worker pools, one model copy each) + one local model server.

**Trade-off:** warm workers and the server hold RAM while idle. The idle timeout limits this. Per-call churn goes away.

## 4. Build footprint (separate from runtime)

- **DONE** — `Cargo.toml`: `[profile.release]` is now `lto = "thin"`, `debug = "line-tables-only"`. New `[profile.dist]` inherits release with fat LTO and no debug info, for shipping only.
- **DONE, not verified by a host build** — `.cargo/config.toml`: lld linker flag.
- **DONE** — `target/debug` removed (about 7G freed). Regenerable.
- **IN PROGRESS** — `scripts/build-pipelines.sh` builds the 39 pipelines that have release binaries into one shared dir, `target/pipelines`, one job at a time. At last check 30 were built, 0 failed. The executor candidate path in `src/pipeline/executor.rs` (around line 320) now points there. Per-crate `target/` dirs (about 41) are still present and get deleted only after the shared build is verified.
- **NOT DONE** — host build with thin LTO. This is the build that unblocks ZCode's zsei freeze fix (`src/zsei/mod.rs` guard scoping). Needs the machine quiet and a memory cap.
- **Tool venvs (candidates, not changed):**
  - `tools/visual-mcp/.venv` has about 4.5G of CUDA and triton wheels (nvidia ~3.2G, triton ~0.9G). No NVIDIA GPU is visible on this machine. CPU-only torch is the candidate.
  - `tools/connectome-mcp/.venv` has TensorFlow (1.9G). Its own `.py` files don't import it. Its server runs system `python3`, which lacks pyarrow, so the venv may not be what actually runs. Verify before changing anything.
  - Six venvs share 28 package names with no version conflicts, so a shared venv is technically feasible. The disk saving is small because the bloat is per-venv, not duplicated.

## 5. Decisions needed

- **D1.** Gate size. Proposed: 3 on this 7.6G machine.
- **D2.** Idle unload timeout for heavy workers. Proposed: 10 minutes.
- **D3.** Make the local model server the default when a local model is configured, or keep it opt-in?
- **D4.** Pipelines as one Cargo workspace with one lockfile, to align versions by construction. Yes or no?
- **D5.** Automatic daily dependency upkeep: detect, stage, build, test, promote with rollback. Needs test coverage first (only a few pipelines have contract tests).

## 6. Questions for ZCode

- Which pipelines are heavy-init? The list in §3.2 is a hypothesis, not checked.
- Typical lane count N per AMT batch.
- Does any loop retry a call rejected at the cap, or does the work get dropped?
- Host RSS at idle and under load. Needs a host run.
- Does the zsei lock-scope fix change anything in this design? Expected no, since it touches storage, not pipeline dispatch.

## 7. Not verified

- The current `executor.rs` and `Cargo.toml` edits have not been compiled by a host build.
- Per-pipeline RSS and model-load cost are not measured.
- The connectome server's real interpreter is not confirmed.
