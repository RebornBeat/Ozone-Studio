# Ozone-Studio Contracts & Algorithms

**Everything interchangeable is registered by kind + name. Stores, algorithms,
search strategies, wire protocols — nothing is hardcoded, nothing is a nameless
magic number.**

This document describes the abstraction contracts in Ozone-Studio: what is
swappable, how to register an implementation, how to select one, and the
guarantees the system makes about its own behavior.

---

## The principle

Every layer of the system is a **contract with a default implementation**:

```
CALLER                    CONTRACT                    BACKENDS
─────────────────────     ───────────────────────     ─────────────────────────
Orchestrator        ───▶  StoreAccess            ───▶  ZSEI (container store)
                                                        (any other store)
Orchestrator        ───▶  PipelineExecutor       ───▶  PipelineRegistry
                                                        (remote dispatch / spawn)
Host ↔ Pipelines    ───▶  JSON envelope          ───▶  any language, any machine
Pipelines / Host    ───▶  K-algorithm registry   ───▶  registered strategies
Model traffic       ───▶  Pipeline 9 contract    ───▶  Anthropic / Chat Completions
```

A contract is the *only* thing a caller sees. Swapping a backend means
implementing the contract — the caller (orchestrator, pipeline, dashboard)
never changes.

---

## 1. StoreAccess — the store of the store

**Kind:** store backend
**Contract:** `src/orchestrator/mod.rs` (`pub trait StoreAccess`)
**Default backend:** `src/orchestrator/adapters.rs` (`ZseiStoreAdapter`)

The orchestrator talks to an abstract store: untyped JSON records in/out
(`query`, `traverse`, `create_container`, `update_container`, `get_container`,
`search_by_keywords`, `get_categories`). Container semantics are a **ZSEI
implementation detail**, not part of the contract. ZSEI is the first selectable
backend — it is never dropped, and it is never privileged: any store that
implements `StoreAccess` swaps in by constructing the orchestrator with a
different adapter.

```rust
let store: Arc<dyn StoreAccess> = Arc::new(ZseiStoreAdapter { zsei });
// …or any other backend:
let store: Arc<dyn StoreAccess> = Arc::new(MyPostgresStore::new(…));
let orchestrator = PromptOrchestrator::new(executor, store, tasks, index);
```

> **Wire note:** all store writes route through the `ZSEIQuery` enum (the
> standardized store call), which allocates IDs, links parents, and seeds
> version history. A different backend maps the same calls to its own
> primitives.

---

## 2. PipelineExecutor + the connect model

**Kind:** pipeline execution
**Contract:** `src/orchestrator/mod.rs` (`pub trait PipelineExecutor`)
**Default backend:** `src/orchestrator/adapters.rs` (`RegistryExecutorAdapter`)
**Dispatch:** `src/pipeline/remote.rs` (`RemotePipelines`)

Pipelines are **independent units** — own crates, own builds, own tests. They
reach the host in two ways, checked in this order:

1. **Connect (primary).** A pipeline boots anywhere in any language, announces
   itself at `POST /pipelines/register` (id, name, execute URL), re-announces
   on a heartbeat, and answers `POST <execute_url>` with the JSON envelope.
   The host dispatches to live connections first — it never spawns over one.
   List: `GET /pipelines/remote` · Deregister: `POST /pipelines/unregister`.
2. **Spawn (fallback).** Unregistered ids fall back to the configured spawn
   convention — a generic launcher, never per-pipeline host code.

Embedding serve mode in a Rust pipeline (std-only, no deps):

```rust
#[path = "../../shared/ozone_serve.rs"]
mod ozone_serve;

// in main():
if let Some(opts) = ozone_serve::serve_mode() {          // --serve [--port N] [--register URL]
    ozone_serve::serve(opts, PIPELINE_ID, NAME.to_string(), handler);
}
```

One-shot CLI mode always works alongside: `--input '<PipelineInput JSON>'`,
one JSON object on stdout. Both paths speak the same envelope (`data` wrapper
unwrapped, same output shape).

---

## 3. The K-algorithm registry — algorithms by kind

**Kind taxonomy + preset registry:** `shared/contracts/k_registry.rs`
**Assurance contract:** `shared/contracts/k_validation.rs`
**Loop-discipline contract:** `shared/contracts/k_loops.rs`
**Search strategies:** `src/zsei/search.rs`
**Host facade:** `src/k_registry.rs` (`KAlgorithms::global()`)

| Kind | Family | Shipped implementations | Default |
|---|---|---|---|
| `validation` | assurance primitives | `strict-5`, `lenient-3`, `paranoid-7` (consecutive YES/NO) | `strict-5` |
| `ordered_loop` | 1×1 loop discipline | `two-strikes`, `three-strikes` (misses before exhaustion) | `two-strikes` |
| `pairwise` | pairwise passes | `default` (window 8, 50 pairs), `wide` (16, 100) | `default` |
| `convergence` | refinement bounds | `fast` (2 passes), `deep` (5) | `fast` |
| `search` | store search strategies | `scan` (substring, default), `exact` (whole-term) | `scan` |

Selecting is data, not code:

```rust
let k = KAlgorithms::global();
let policy = k.validation.get(Some("paranoid-7"));       // a named preset…
let pairwise = k.pairwise.default_preset().clone();      // …or the default
k.search.set_default("exact").await;                     // strategy selection
```

Registering a new search strategy:

```rust
struct TraversalSearch;
impl SearchStrategy for TraversalSearch {
    fn id(&self) -> &'static str { "traversal" }
    fn search(&self, storage, keywords, type_filter) -> OzoneResult<Vec<ContainerID>> { … }
}
KAlgorithms::global().search.register(Arc::new(TraversalSearch)).await;
```

Value-type families (the policy presets) use `NamedPresets<T>` from the shared
contracts — pipelines embed the same files via `#[path]`, so the taxonomy and
defaults are identical everywhere.

> **Single-source guarantee:** shared contracts (`shared/contracts/*.rs`) are
> included — never copied — by every consumer. One edit, all consumers; the
> contracts' unit tests run in each embedding crate.

---

## 4. Model calls — the PIPELINE-9 MODEL-CALL CONTRACT

The orchestrator speaks exactly one model-call shape — the **pipeline-9
model-call contract** (`assets/pipelines/general/prompt/main.rs`, named at the
top of the file):

```
IN : { prompt, system_prompt?, temperature?, max_tokens?, … }
OUT: { response, model_used, tokens_used?, finish_reason?, … }
```

…always through the metered path so the token budget stays truthful.

**Wire protocols are adapters BEHIND this contract**, selected by config
(`ModelConfig.wire_protocol`), never by callers:

| `wire_protocol` | Wire | Notes |
|---|---|---|
| `"anthropic"` | Anthropic Messages (`/v1/messages`) | system as top-level field, `x-api-key` + `anthropic-version` headers |
| `"chat_completions"` | OpenAI Chat Completions (`/chat/completions`) | system as a message, `Bearer` auth, `choices[0]` |
| unset | sniffed from the endpoint URL | backward compatible |

New protocols (Gemini, local runtimes) implement the same contract and add a
`WireProtocol` variant — callers never change. The orchestrator is untouched
when wire protocols change: protocol is a pipeline-9 concern, never a caller
concern.

---

## 5. Honesty guarantees

These hold across every contract above:

1. **No fabricated confidence.** Scores are either *captured* from the source
   that measured them, or *derived from verification* (a node is `verified`
   iff it carries source provenance). Absence is `None`/`false`/`0.0` — never
   an invented default like `0.8`.
2. **Policies are named, not scattered.** Loop exhaustion (2 strikes), pairwise
   windows (8), pair caps (50), convergence bounds (2 passes), validation
   strength (5) — all live as named presets in the registry. Bodies reference
   the policy, never the number.
3. **Failures are data.** A store search that finds nothing returns empty; a
   pipeline that fails returns `success: false` + `error` as JSON. Failures
   don't crash callers and callers don't invent successes.
4. **Metrics are preserved.** Contract reshapes *union* with existing fields —
   `use_count`, `success_rate`, file paths, telemetry fields survive; nothing
   tracked is silently dropped.

---

## File map

| Area | Files |
|---|---|
| Store contract | `src/orchestrator/mod.rs` (trait), `src/orchestrator/adapters.rs` (backends) |
| Pipeline dispatch | `src/pipeline/remote.rs`, `src/pipeline/executor.rs`, `src/pipeline/mod.rs` |
| K taxonomy + presets | `shared/contracts/k_registry.rs` |
| K validation | `shared/contracts/k_validation.rs` |
| K loop discipline | `shared/contracts/k_loops.rs` |
| K search strategies | `src/zsei/search.rs` |
| Host facade | `src/k_registry.rs` |
| Pipeline serve mode | `assets/pipelines/shared/ozone_serve.rs` |
| Crate generation | `tools/gen_pipeline_crates.py` · triage: `tools/PIPELINE_TRIAGE.md` |
| Register endpoints | `src/grpc/mod.rs` (`/pipelines/register`, `/pipelines/unregister`, `/pipelines/remote`) |

---

## Status legends: ✓ live · ◐ partial · ○ planned

| Layer | Status |
|---|---|
| StoreAccess + ZSEI adapter | ✓ |
| PipelineExecutor (remote-first, spawn fallback) | ✓ |
| Connect model (register/heartbeat/serve) | ✓ (text, task_manager, context_viewer) |
| K-validation registry | ✓ |
| K-loops registry (ordered/pairwise/convergence) | ✓ |
| K-search registry (scan/exact; traversal/embedding to follow) | ✓ |
| Wire protocol adapters (pipeline 9) | ○ |
| Additional store backends | ○ |
| Config-driven default selection | ○ |
| Orchestrator decomposition into modules | ○ (standardization complete — safe to carve) |
