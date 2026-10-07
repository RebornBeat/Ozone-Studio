//! Process-wide context windows, and the OpenRouter live catalog.
//!
//! Two sources, one lookup (`window_for`):
//! - The config registry, populated from config (`OzoneConfig::load` and
//!   `save`). A per-model `[[models.available_models]]` entry always wins
//!   over the global `[models]` block.
//! - The OpenRouter catalog, from GET https://openrouter.ai/api/v1/models.
//!   It is fetched at host start, and refetched when a served OpenRouter id
//!   is missing from it (at most once per 10 minutes). For an id the catalog
//!   knows, the catalog's `context_length` is the window and wins over the
//!   config registry, because a config value for a router id is a guess.
//!
//! A lookup miss is `None`, never a guessed number: callers record the window
//! as unknown and the miss is named once in the log.
//!
//! Pool: `[models] openrouter_pool` ("off" | "free"; "paid" is refused) and
//! `openrouter_pool_size` are published from config at load and save. The
//! walk appends that many concrete catalog models as fallback candidates
//! (see `catalog_pool`).

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const OPENROUTER_MODELS_URL: &str = "https://openrouter.ai/api/v1/models";
const CATALOG_FETCH_TIMEOUT: Duration = Duration::from_secs(20);
const CATALOG_REFETCH_MIN_INTERVAL_MS: u64 = 10 * 60 * 1000;
const CATALOG_FILE: &str = "model_catalog.json";

static WINDOWS: OnceLock<RwLock<HashMap<String, u32>>> = OnceLock::new();
static WARNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
static CATALOG: OnceLock<RwLock<Option<Catalog>>> = OnceLock::new();
static POOL: OnceLock<RwLock<(String, usize)>> = OnceLock::new();
static LAST_CATALOG_ATTEMPT_MS: AtomicU64 = AtomicU64::new(0);

/// One OpenRouter model as the catalog reports it. `is_free` is true only
/// when both prompt and completion prices parse to exactly zero; a missing
/// price is not treated as free.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CatalogEntry {
    pub id: String,
    pub context_length: u32,
    pub max_completion_tokens: Option<u32>,
    pub is_free: bool,
}

struct Catalog {
    entries: HashMap<String, CatalogEntry>,
}

#[derive(Serialize, Deserialize)]
struct PersistedCatalog {
    fetched_at_ms: u64,
    entries: Vec<CatalogEntry>,
}

fn table() -> &'static RwLock<HashMap<String, u32>> {
    WINDOWS.get_or_init(|| RwLock::new(HashMap::new()))
}

fn catalog_cell() -> &'static RwLock<Option<Catalog>> {
    CATALOG.get_or_init(|| RwLock::new(None))
}

fn pool_cell() -> &'static RwLock<(String, usize)> {
    POOL.get_or_init(|| RwLock::new(("off".to_string(), 3)))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Replace the registry. `per_model` entries are inserted first and always
/// win; `global` entries fill only keys nothing else claimed.
pub fn replace(per_model: Vec<(String, u32)>, global: Vec<(String, u32)>) {
    let mut map: HashMap<String, u32> = HashMap::new();
    for (key, window) in per_model {
        map.entry(key).or_insert(window);
    }
    for (key, window) in global {
        map.entry(key).or_insert(window);
    }
    let mut guard = table().write().unwrap_or_else(|e| e.into_inner());
    *guard = map;
}

/// Publish the OpenRouter pool setting from `[models]`. An unknown mode is
/// named and treated as "off", so a typo never silently adds candidates.
pub fn set_pool(mode: &str, size: usize) {
    let mode: &str = match mode {
        "free" => "free",
        "off" => "off",
        // Paid models are not used under the operator's policy. A paid pool is
        // refused here, so no value of the setting can reach a paid candidate.
        "paid" => {
            tracing::warn!(
                "[models] openrouter_pool = \"paid\" is refused: paid OpenRouter models are not used. Treating as \"off\""
            );
            "off"
        }
        other => {
            tracing::warn!(
                mode = %other,
                "[models] openrouter_pool must be \"off\" or \"free\" — treating as \"off\""
            );
            "off"
        }
    };
    *pool_cell().write().unwrap_or_else(|e| e.into_inner()) = (mode.to_string(), size);
}

/// The published pool setting: (mode, size).
pub fn pool_settings() -> (String, usize) {
    pool_cell().read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// The catalog entry for an exact OpenRouter model id, if the catalog has one.
pub fn catalog_entry(id: &str) -> Option<CatalogEntry> {
    let guard = catalog_cell().read().unwrap_or_else(|e| e.into_inner());
    guard.as_ref()?.entries.get(id).cloned()
}

/// The live catalog window for an exact OpenRouter model id.
pub fn catalog_window(id: &str) -> Option<u32> {
    catalog_entry(id).map(|e| e.context_length)
}

/// Whether a configured model is served through OpenRouter.
pub fn is_openrouter(m: &crate::config::AvailableModel) -> bool {
    m.api_endpoint.as_deref().is_some_and(|e| e.contains("openrouter.ai"))
}

/// Concrete catalog models for the pool. Only `free` selects anything, and it
/// keeps is_free entries. Router ids (`openrouter/...`) are never pool members.
/// Ids in `exclude` (already in the configured chain) are skipped before the
/// size cut. `free_only` drops paid entries. Order: context_length
/// descending, then id ascending, so the largest known window is tried first.
pub fn catalog_pool(
    mode: &str,
    size: usize,
    exclude: &HashSet<String>,
    free_only: bool,
) -> Vec<CatalogEntry> {
    // Only "free" selects anything. Any other value selects nothing, so a
    // paid pool cannot be reached by passing an unexpected mode string.
    if size == 0 || mode != "free" {
        return Vec::new();
    }
    let guard = catalog_cell().read().unwrap_or_else(|e| e.into_inner());
    let Some(cat) = guard.as_ref() else {
        return Vec::new();
    };
    let mut picked: Vec<CatalogEntry> = cat
        .entries
        .values()
        .filter(|e| !e.id.starts_with("openrouter/"))
        .filter(|e| !exclude.contains(&e.id))
        .filter(|e| e.is_free)
        .filter(|e| !free_only || e.is_free)
        .cloned()
        .collect();
    picked.sort_by(|a, b| {
        b.context_length
            .cmp(&a.context_length)
            .then_with(|| a.id.cmp(&b.id))
    });
    picked.truncate(size);
    // LEDGER-AWARE ORDER (found by walk #3's attempt trail: the pool tried
    // google/lyria-3 — a MUSIC model with a 1M window — for text generation,
    // burning attempts on errors/empties). Window size alone is not
    // suitability; the ledger's historical success rate is. Ledger rank
    // first (known-good surfaces, known-bad sinks, unknowns neutral), window
    // keeps its role as the pre-ranking so big-window models get the chance
    // to build history.
    let ranked = crate::model_ledger::rank(&picked.iter().map(|e| e.id.clone()).collect::<Vec<_>>());
    let mut order: HashMap<String, usize> = HashMap::new();
    for (i, id) in ranked.into_iter().enumerate() {
        order.insert(id, i);
    }
    picked.sort_by(|a, b| {
        let ra = order.get(&a.id).copied().unwrap_or(usize::MAX);
        let rb = order.get(&b.id).copied().unwrap_or(usize::MAX);
        ra.cmp(&rb).then_with(|| a.id.cmp(&b.id))
    });
    picked
}

/// The registered window for a model id (or a served id), or `None`. The
/// OpenRouter catalog is consulted first for ids it knows.
pub fn window_for(model: &str) -> Option<u32> {
    if let Some(w) = catalog_window(model) {
        return Some(w);
    }
    let guard = table().read().unwrap_or_else(|e| e.into_inner());
    if let Some(w) = guard.get(model) {
        return Some(*w);
    }
    let (_, rest) = model.split_once(':')?;
    guard.get(rest).copied()
}

/// Name an unregistered model id once per process, so a missing window is
/// visible without flooding the log on every call.
pub fn note_unknown(model: &str) {
    let warned = WARNED.get_or_init(|| Mutex::new(HashSet::new()));
    let mut set = warned.lock().unwrap_or_else(|e| e.into_inner());
    if set.insert(model.to_string()) {
        tracing::warn!(
            model = %model,
            "context window unknown for this model id — no registered available_models entry, local_model_path, or OpenRouter catalog entry matches it; the call record keeps window 0"
        );
    }
}

fn price(v: Option<&serde_json::Value>) -> Option<f64> {
    match v? {
        serde_json::Value::String(s) => s.parse().ok(),
        serde_json::Value::Number(n) => n.as_f64(),
        _ => None,
    }
}

/// Parse an OpenRouter /models body. An entry with no usable context_length
/// is skipped and counted, never given an invented window.
pub fn parse_catalog(body: &serde_json::Value) -> Result<(Vec<CatalogEntry>, usize), String> {
    let items = body
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| "response has no data array".to_string())?;
    let mut out = Vec::with_capacity(items.len());
    let mut skipped = 0usize;
    for item in items {
        let Some(id) = item.get("id").and_then(|v| v.as_str()) else {
            skipped += 1;
            continue;
        };
        let Some(ctx) = item
            .get("context_length")
            .and_then(|v| v.as_u64())
            .and_then(|v| u32::try_from(v).ok())
        else {
            skipped += 1;
            continue;
        };
        let max_completion_tokens = item
            .get("top_provider")
            .and_then(|t| t.get("max_completion_tokens"))
            .and_then(|v| v.as_u64())
            .and_then(|v| u32::try_from(v).ok());
        let pricing = item.get("pricing");
        let is_free = matches!(
            (
                price(pricing.and_then(|p| p.get("prompt"))),
                price(pricing.and_then(|p| p.get("completion"))),
            ),
            (Some(a), Some(b)) if a == 0.0 && b == 0.0
        );
        out.push(CatalogEntry {
            id: id.to_string(),
            context_length: ctx,
            max_completion_tokens,
            is_free,
        });
    }
    Ok((out, skipped))
}

/// Fetch the live catalog. Async reqwest on the tokio runtime: the caller's
/// thread is never blocked. Sends no credentials.
async fn fetch_live() -> Result<(Vec<CatalogEntry>, usize), String> {
    let client = reqwest::Client::builder()
        .timeout(CATALOG_FETCH_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(OPENROUTER_MODELS_URL)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    parse_catalog(&body)
}

fn install(entries: Vec<CatalogEntry>) {
    let entries = entries.into_iter().map(|e| (e.id.clone(), e)).collect();
    *catalog_cell().write().unwrap_or_else(|e| e.into_inner()) = Some(Catalog { entries });
}

fn catalog_path() -> std::path::PathBuf {
    let data_dir =
        std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    std::path::PathBuf::from(data_dir).join(CATALOG_FILE)
}

/// Fetch and persist the live catalog. If the fetch fails and nothing is in
/// memory yet, load the persisted copy and say how old it is. If neither
/// exists, the windows stay unknown; no value is invented.
pub async fn refresh_catalog() {
    match fetch_live().await {
        Ok((entries, skipped)) => {
            let free = entries.iter().filter(|e| e.is_free).count();
            if skipped > 0 {
                tracing::warn!(
                    skipped,
                    "OpenRouter catalog: entries without a usable context_length skipped (no window invented)"
                );
            }
            tracing::info!(models = entries.len(), free, "OpenRouter catalog loaded (live)");
            let path = catalog_path();
            let persisted = PersistedCatalog {
                fetched_at_ms: now_ms(),
                entries: entries.clone(),
            };
            match serde_json::to_vec_pretty(&persisted) {
                Ok(bytes) => {
                    if let Some(dir) = path.parent() {
                        let _ = tokio::fs::create_dir_all(dir).await;
                    }
                    if let Err(e) = tokio::fs::write(&path, bytes).await {
                        tracing::warn!(
                            path = %path.display(),
                            error = %e,
                            "OpenRouter catalog: could not persist the live copy"
                        );
                    }
                }
                Err(e) => tracing::warn!(
                    error = %e,
                    "OpenRouter catalog: could not serialize the live copy"
                ),
            }
            install(entries);
        }
        Err(e) => {
            tracing::warn!(error = %e, "OpenRouter catalog: live fetch failed");
            let have = catalog_cell()
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .is_some();
            if !have {
                load_persisted().await;
            }
        }
    }
}

async fn load_persisted() {
    let path = catalog_path();
    let bytes = match tokio::fs::read(&path).await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "OpenRouter catalog: no persisted copy — OpenRouter context windows stay unknown"
            );
            return;
        }
    };
    match serde_json::from_slice::<PersistedCatalog>(&bytes) {
        Ok(p) => {
            let age_minutes = now_ms().saturating_sub(p.fetched_at_ms) / 60_000;
            tracing::warn!(
                age_minutes,
                models = p.entries.len(),
                "OpenRouter catalog: using the persisted copy (live fetch failed)"
            );
            install(p.entries);
        }
        Err(e) => tracing::warn!(
            path = %path.display(),
            error = %e,
            "OpenRouter catalog: persisted copy is unreadable — OpenRouter context windows stay unknown"
        ),
    }
}

/// Start the catalog fetch in the background, so host start is not blocked.
/// Must be called from inside the tokio runtime.
pub fn spawn_catalog_refresh() {
    LAST_CATALOG_ATTEMPT_MS.store(now_ms(), Ordering::SeqCst);
    tokio::spawn(refresh_catalog());
}

/// Refetch the catalog because a served OpenRouter id is missing from it.
/// At most once per 10 minutes: the timestamp is claimed with a
/// compare-and-swap, so concurrent callers start at most one fetch. No-op
/// outside a tokio runtime.
pub fn request_refresh_if_stale() {
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    let last = LAST_CATALOG_ATTEMPT_MS.load(Ordering::SeqCst);
    let now = now_ms();
    if last != 0 && now.saturating_sub(last) < CATALOG_REFETCH_MIN_INTERVAL_MS {
        return;
    }
    if LAST_CATALOG_ATTEMPT_MS
        .compare_exchange(last, now, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    tokio::spawn(refresh_catalog());
}
