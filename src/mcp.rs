//! MCP tool registry — the connection surface for external tools.
//!
//! Ozone-Studio hosts tools the way it hosts agents and models: they
//! register themselves and appear in one observable registry. This module is
//! the wire-level foundation for MCP (Model Context Protocol) connections —
//! the planned 90+ tool list, the gamedev tooling, anything that speaks
//! tool. Registration is the contract; the JSON-RPC handshake layer attaches
//! to the same records as transports come online (stdio | http | sse).
//!
//! Tools register with:
//!   POST /mcp/tools/register  {name, transport, endpoint, capabilities?, server_version?}
//! and list with:
//!   GET  /mcp/tools
//!
//! Every registration and removal is captured as monitor activity, so the
//! multi-track environment (browser plugin, ZCode, phone, desktop) sees tool
//! landings in the same feed as everything else.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

/// Transport a tool speaks over. `stdio` tools are child processes on the
/// host; `http`/`sse` tools live at a URL.
#[derive(Debug, Clone, PartialEq, Eq, std::hash::Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpTransport {
    Stdio,
    Http,
    Sse,
    /// A participating MCP AGENT is the provider (endpoint `agent:<name>` or `agent:*`). A call creates a durable
    /// request and AWAITS the agent's report — see `src/agent_requests.rs`.
    Agent,
}

impl McpTransport {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "stdio" => Some(Self::Stdio),
            "http" => Some(Self::Http),
            "sse" => Some(Self::Sse),
            "agent" => Some(Self::Agent),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stdio => "stdio",
            Self::Http => "http",
            Self::Sse => "sse",
            Self::Agent => "agent",
        }
    }
}

/// One registered tool.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpTool {
    pub name: String,
    pub transport: McpTransport,
    /// Command+args (stdio) or URL (http/sse).
    pub endpoint: String,
    pub capabilities: Vec<String>,
    pub server_version: Option<String>,
    pub registered_at: u64,
    /// OPERATOR-CONTROLLED (2026-10-18): whether this tool's backing
    /// process should be started automatically at host boot and kept
    /// alive by the crash-supervisor (`src/mcp_supervisor.rs`). Default
    /// `false` for every tool, including the 134 already persisted —
    /// this is an opt-in the operator makes per tool via the UI, never
    /// guessed or pre-decided here. One OS process can back many
    /// registered capability names (confirmed live: `bridges` alone
    /// backs 67 entries sharing one `(transport, endpoint)` pair) —
    /// toggling this on any one entry in that group sets it for the
    /// whole group via `McpRegistry::set_auto_start`, so the group can
    /// never desync into a half-on state.
    #[serde(default)]
    pub auto_start: bool,
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// ── Process-global handles ──────────────────────────────────────────────────
// The registry + ledger are built once in the server setup and shared via
// AppState; this static makes them reachable from anywhere in the process
// (orchestrator stages, AMT loops) without threading Arcs through every
// constructor — the same pattern as GraphEventHub::global and
// KAlgorithms::global.

static GLOBAL_MCP: OnceLock<(Arc<McpRegistry>, Arc<UsageLedger>)> = OnceLock::new();

/// Install the process-global handles. Called once at server startup; later
/// calls are no-ops (first install wins).
pub fn install_global(registry: Arc<McpRegistry>, usage: Arc<UsageLedger>) {
    let _ = GLOBAL_MCP.set((registry, usage));
}

/// Compact capability summary for prompt embedding (§11.2): one line per
/// registered tool — name (version): first capability. Empty string when
/// nothing is registered. Read-only; never blocks registration.
pub async fn global_registry_summary() -> Option<String> {
    let (registry, _usage) = GLOBAL_MCP.get()?;
    let tools = registry.list().await;
    if tools.is_empty() {
        return None;
    }
    let lines: Vec<String> = tools
        .iter()
        .map(|t| {
            let cap = t.capabilities.first().cloned().unwrap_or_default();
            format!("- {} ({}): {}", t.name, t.server_version.clone().unwrap_or_default(), cap)
        })
        .collect();
    Some(lines.join("\n"))
}

/// Structured registry snapshot for in-process aggregation (Stage 3 tools/
/// MCP gathering — operator: "gather mcps and tools where pipeline gathering
/// is as well"). None when the global handles aren't installed.
pub async fn registry_entries() -> Option<Vec<McpTool>> {
    let (registry, _usage) = GLOBAL_MCP.get()?;
    let tools = registry.list().await;
    if tools.is_empty() {
        None
    } else {
        Some(tools)
    }
}

/// Register one tool through the process-global registry (the same `McpRegistry::register` the `/mcp/tools/register` route uses, so it is
/// persisted and visible to every reader). `None` = the global handles are not installed yet. Used by `orchestrator/tool_creation.rs`, which
/// registers ONLY stdio rows (the host never spawns a stdio tool — see `McpTransport::Stdio` in `invoke`).
pub async fn register_global(tool: McpTool) -> Option<bool> {
    let (registry, _usage) = GLOBAL_MCP.get()?;
    Some(registry.register(tool).await)
}

/// Issue the standardized abstract call through the process-global handles.
/// Same metering, gating, and rippling as the HTTP path.
/// Process-global jurisdiction gate for IN-PROCESS tool calls (2026-11-02 audit).
/// The HTTP `/mcp/call` handler runs the full jurisdiction gate before `invoke`,
/// but `call_global` callers (tool routing, the sensor tick, approved assistant
/// proposals) went straight to the registry and bypassed it. Installed at boot
/// (and refreshed on rule reload) from the same rule snapshot as the per-step
/// gate; `None` = not installed (logged once, call proceeds — same posture as
/// the step gate, never silently claiming a check ran).
type CallGate = Arc<dyn Fn(&str) -> Result<(), String> + Send + Sync>;
static CALL_GATE: OnceLock<std::sync::RwLock<Option<CallGate>>> = OnceLock::new();

pub fn set_call_gate(gate: Option<CallGate>) {
    let cell = CALL_GATE.get_or_init(|| std::sync::RwLock::new(None));
    if let Ok(mut g) = cell.write() {
        *g = gate;
    }
}

fn installed_call_gate() -> Option<CallGate> {
    CALL_GATE.get()?.read().ok()?.clone()
}

/// Check `text` against the installed jurisdiction call gate, recording the
/// verdict. `Ok` when allowed or when no gate is installed yet (warned once).
/// Shared by `call_global` (in-process tool calls) and the external
/// `/pipeline/execute` route, so neither path can skip jurisdiction.
pub fn check_call_gate(span_kind: &str, subject: &str, text: &str) -> Result<(), String> {
    let verdict = check_call_gate_inner(span_kind, subject, text);
    // AUDIT (jurisdiction deepening): every call-gate run is an append-only decision row — hashes and the entry-point label only.
    let pol = crate::orchestrator::jurisdiction::policy::load_policy();
    let h = blake3::hash(format!("{subject} {text}").as_bytes()).to_hex().to_string();
    crate::orchestrator::jurisdiction::policy::record_decision(
        span_kind,
        &h[..32],
        None,
        &pol,
        "",
        if verdict.is_ok() { "allowed" } else { "blocked" },
        &[],
        "call gate (Block rules only; rule snapshot refreshed every 10 min or the compiled Global set when none is installed)",
    );
    verdict
}

fn check_call_gate_inner(span_kind: &str, subject: &str, text: &str) -> Result<(), String> {
    match installed_call_gate() {
        Some(gate) => {
            // WHOLE text (no-cap doctrine, 2026-11-04): this used to inspect only the first 4,000 characters, so a Block phrase
            // later in a large call/pipeline input was never checked — a gate must see everything it gates.
            let hay: String = format!("{} {}", subject, text);
            if let Err(reason) = gate(&hay) {
                crate::orchestrator::entry_gate::record_gate_verdict(span_kind, subject, "blocked", Some(&reason), None);
                return Err(reason);
            }
            Ok(())
        }
        None => {
            // NEVER FAIL OPEN (jurisdiction deepening): before the loaded-rule gate is installed the Global layer is still unconditional,
            // so the compiled global set judges the call instead of letting it through unchecked.
            static WARNED: OnceLock<()> = OnceLock::new();
            if WARNED.set(()).is_ok() {
                tracing::warn!("no jurisdiction call gate installed yet — the compiled Global rule set judges calls until the loaded rules are installed");
            }
            let hay: String = format!("{} {}", subject, text);
            if let Err(reason) = crate::orchestrator::jurisdiction::step_gate_check(
                &crate::orchestrator::jurisdiction::compiled_global_fallback_rules(),
                &hay,
            ) {
                crate::orchestrator::entry_gate::record_gate_verdict(span_kind, subject, "blocked", Some(&reason), None);
                return Err(reason);
            }
            Ok(())
        }
    }
}

/// POST-CALL HOOK (2026-11-04): `/mcp/call` persists a tool's `graph` block as containers and writes the S13 `tool_calls.jsonl` capture row;
/// the orchestrator's in-process path (`call_global`) did neither, so provider steps' results never reached the graph and their calls left no
/// capture row. The host installs ONE hook at boot (`install_post_call_hook`, lib.rs) that does both with `called_from` provenance. It runs for
/// EVERY call (success or failure) and receives (tool, agent, call context, success, error, output).
pub type PostCallHook = std::sync::Arc<
    dyn Fn(String, String, Option<serde_json::Value>, bool, Option<String>, Option<serde_json::Value>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;
static POST_CALL_HOOK: std::sync::OnceLock<PostCallHook> = std::sync::OnceLock::new();

/// Install the post-call hook (first install wins).
pub fn install_post_call_hook(hook: PostCallHook) {
    let _ = POST_CALL_HOOK.set(hook);
}

pub async fn call_global(call: McpCall) -> McpResult {
    let input_text = serde_json::to_string(&call.input).unwrap_or_default();
    if let Err(reason) = check_call_gate("mcp-global", &call.tool, &input_text) {
        return McpResult {
            success: false,
            output: None,
            error: Some(format!("blocked by the jurisdiction gate: {}", reason)),
            usage: serde_json::json!({ "agent": call.agent, "tool": call.tool, "allowed": false, "gate": "jurisdiction" }),
        };
    }
    let Some((registry, usage)) = GLOBAL_MCP.get() else {
        return McpResult {
            success: false,
            output: None,
            error: Some("mcp global handles not installed — install_global has not run yet".to_string()),
            usage: serde_json::json!({ "agent": call.agent, "tool": call.tool, "allowed": false }),
        };
    };
    let (hook_tool, hook_agent, hook_ctx) = (call.tool.clone(), call.agent.clone(), call.context.clone());
    let result = registry.invoke(call, usage, None).await;
    if let Some(hook) = POST_CALL_HOOK.get() {
        hook(hook_tool, hook_agent, hook_ctx, result.success, result.error.clone(), result.output.clone()).await;
    }
    result
}

/// Validated caller identity forwarded to tool servers (see
/// `McpRegistry::invoke_with_identity`). The default is "unvalidated", the
/// documented localhost posture. Role allowlists downstream MUST use
/// `validated`/`session_user_id` (surfaced as `agent_trust` +
/// `session_user_id` in the envelope and `identity_validated` in the result
/// usage) and fall back to the declared `agent` string ONLY when
/// `validated` is false — in which case the result says so.
#[derive(Debug, Clone, Default)]
pub struct McpCallerIdentity {
    pub validated: bool,
    pub session_user_id: Option<String>,
}

/// The registry itself — one per host, shared through AppState.
#[derive(Default)]
pub struct McpRegistry {
    tools: RwLock<HashMap<String, McpTool>>,
    /// When set, register/unregister rewrite this file and `load_persisted`
    /// restores it at boot (registry-persistence fix, 2026-09-28).
    persist_path: Option<String>,
}

impl McpRegistry {
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
            persist_path: None,
        }
    }

    /// Register (or idempotently refresh) a tool by name.
    pub async fn register(&self, tool: McpTool) -> bool {
        let replaced = self.tools.read().await.contains_key(&tool.name);
        self.tools.write().await.insert(tool.name.clone(), tool);
        self.persist().await;
        replaced
    }

    pub async fn unregister(&self, name: &str) -> bool {
        let removed = self.tools.write().await.remove(name).is_some();
        if removed {
            self.persist().await;
        }
        removed
    }

    /// PERSISTED REGISTRATIONS (fixed 2026-09-28, gap found live: the
    /// registry was in-memory, so every host restart wiped all registered
    /// tools — the 67 bridge tools + 2 terminal tools vanished and had to
    /// be re-registered by hand). When a persist path is configured, every
    /// register/unregister rewrites the file, and `load_persisted` restores
    /// it at boot. External tools self-heal across restarts like every
    /// other registry in this codebase.
    pub fn with_persistence(mut self, path: String) -> Self {
        self.persist_path = Some(path);
        self
    }

    async fn persist(&self) {
        if let Some(path) = &self.persist_path {
            let tools = self.tools.read().await;
            let list: Vec<&McpTool> = tools.values().collect();
            if let Ok(json) = serde_json::to_string_pretty(&list) {
                // Durable replace: the registered-tools file is retried until it is written.
                crate::durable_write::replace(path, json.into_bytes());
            }
        }
    }

    pub async fn load_persisted(&self) -> usize {
        let Some(path) = &self.persist_path else { return 0 };
        let Ok(content) = std::fs::read_to_string(path) else { return 0 };
        let Ok(tools) = serde_json::from_str::<Vec<McpTool>>(&content) else { return 0 };
        let count = tools.len();
        let mut map = self.tools.write().await;
        for tool in tools {
            map.entry(tool.name.clone()).or_insert(tool);
        }
        count
    }

    pub async fn get(&self, name: &str) -> Option<McpTool> {
        self.tools.read().await.get(name).cloned()
    }

    pub async fn list(&self) -> Vec<McpTool> {
        let mut tools: Vec<McpTool> = self.tools.read().await.values().cloned().collect();
        tools.sort_by(|a, b| a.name.cmp(&b.name));
        tools
    }

    /// Toggle `auto_start` for every entry sharing `name`'s (transport,
    /// endpoint) pair — the real process-identity unit (see the doc
    /// comment on `McpTool::auto_start`). Returns the names actually
    /// affected, or `None` if `name` isn't registered at all.
    pub async fn set_auto_start(&self, name: &str, enabled: bool) -> Option<Vec<String>> {
        let mut tools = self.tools.write().await;
        let key = {
            let t = tools.get(name)?;
            (t.transport.clone(), t.endpoint.clone())
        };
        let mut affected = Vec::new();
        for t in tools.values_mut() {
            if t.transport == key.0 && t.endpoint == key.1 {
                t.auto_start = enabled;
                affected.push(t.name.clone());
            }
        }
        drop(tools);
        self.persist().await;
        Some(affected)
    }

    /// Distinct `(transport, endpoint)` groups with `auto_start == true` —
    /// the real unit the supervisor starts/stops/watches, since one OS
    /// process can back many registered capability names.
    pub async fn auto_start_groups(&self) -> Vec<(McpTransport, String, Vec<String>)> {
        let tools = self.tools.read().await;
        let mut groups: HashMap<(McpTransport, String), Vec<String>> = HashMap::new();
        for t in tools.values() {
            if t.auto_start {
                groups
                    .entry((t.transport.clone(), t.endpoint.clone()))
                    .or_default()
                    .push(t.name.clone());
            }
        }
        groups
            .into_iter()
            .map(|((transport, endpoint), names)| (transport, endpoint, names))
            .collect()
    }
}

// ── Usage ledger ────────────────────────────────────────────────────────────
// Every MCP tool call an agent makes is recorded here — per agent, per UTC
// day, per tool. This is what lets MCP work carry its own measurable budget
// instead of disappearing into an agent's un-metered native calls. A daily
// limit, when configured, is enforced here: over-limit calls are refused by
// the ledger itself, not by convention.

use std::sync::atomic::{AtomicU32, Ordering};

pub struct UsageLedger {
    /// Per-agent daily call cap (0 = unlimited). From OZONE_MCP_DAILY_LIMIT.
    daily_limit: AtomicU32,
    /// (agent, YYYY-MM-DD) → total calls that day.
    daily: RwLock<HashMap<(String, String), u64>>,
    /// (agent, YYYY-MM-DD, tool) → calls that day (breakdown).
    per_tool: RwLock<HashMap<(String, String, String), u64>>,
}

fn utc_today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    utc_date_from_secs(secs)
}

/// Pure date computation, extracted from `utc_today` so the real day-
/// rollover boundary is directly testable without waiting a real day or
/// mocking `SystemTime::now()` (see T-U1's day-rollover test below).
fn utc_date_from_secs(secs: u64) -> String {
    // Civil-from-days (Howard Hinnant's algorithm) — no external deps.
    let days = secs / 86400;
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", y, m, d)
}

impl UsageLedger {
    pub fn new() -> Self {
        let daily_limit = std::env::var("OZONE_MCP_DAILY_LIMIT")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        Self::with_limit(daily_limit)
    }

    /// Explicit-limit constructor (ops scripts + tests).
    pub fn with_limit(daily_limit: u32) -> Self {
        Self {
            daily_limit: AtomicU32::new(daily_limit),
            daily: RwLock::new(HashMap::new()),
            per_tool: RwLock::new(HashMap::new()),
        }
    }

    /// Record one call. Returns (allowed, total_today, limit). An over-limit
    /// call is counted but refused.
    pub async fn record(&self, agent: &str, tool: &str) -> (bool, u64, u32) {
        let day = utc_today();
        let limit = self.daily_limit.load(Ordering::Relaxed);

        let mut daily = self.daily.write().await;
        let key = (agent.to_string(), day.clone());
        let total = daily.entry(key).or_insert(0);
        *total += 1;
        let total = *total;
        drop(daily);

        *self
            .per_tool
            .write()
            .await
            .entry((agent.to_string(), day, tool.to_string()))
            .or_insert(0) += 1;

        let allowed = limit == 0 || total <= limit as u64;
        (allowed, total, limit)
    }

    /// Per-agent snapshot: today's total, limit, remaining, per-tool breakdown.
    pub async fn snapshot(&self, agent: &str) -> serde_json::Value {
        let day = utc_today();
        let limit = self.daily_limit.load(Ordering::Relaxed);
        let total = *self
            .daily
            .read()
            .await
            .get(&(agent.to_string(), day.clone()))
            .unwrap_or(&0);
        let mut per_tool: Vec<(String, u64)> = self
            .per_tool
            .read()
            .await
            .iter()
            .filter(|((a, d, _), _)| a == agent && d == &day)
            .map(|((_, _, t), c)| (t.clone(), *c))
            .collect();
        per_tool.sort_by(|a, b| b.1.cmp(&a.1));
        serde_json::json!({
            "agent": agent,
            "day": day,
            "total_today": total,
            "daily_limit": limit,
            "remaining": if limit == 0 { None } else { Some((limit as u64).saturating_sub(total)) },
            "per_tool": per_tool,
        })
    }
}

impl Default for UsageLedger {
    fn default() -> Self {
        Self::new()
    }
}

// ── The standardized abstract MCP call ──────────────────────────────────────
// Same doctrine as StoreAccess (stores) and WireProtocol (models): ONE
// abstract call type every MCP tool invocation normalizes to, regardless of
// which agent calls, which transport the tool speaks, or where the tool
// runs. The orchestrator, AMT stages, and shared-context flows all issue
// McpCall; the registry meteres, records, and dispatches.

/// One abstract tool invocation. This is the type the whole system speaks —
/// agents over HTTP (`POST /mcp/call`), orchestrator stages in-process, and
/// future transports behind the same shape.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpCall {
    /// Registered tool name (must exist in the McpRegistry).
    pub tool: String,
    /// Calling agent — the usage ledger keys on this ("zcode", "claude-code",
    /// "orchestrator", "phone", …).
    pub agent: String,
    /// Tool-specific arguments (free-form JSON; the tool owns the schema).
    pub input: serde_json::Value,
    /// Optional per-call context (task_id, workspace_id, trace id…).
    #[serde(default)]
    pub context: Option<serde_json::Value>,
    /// CALLER IDENTITY (2026-09-29): an Ed25519 session token from
    /// /auth/authenticate. When present and VALID, the call's identity is
    /// authenticated — the agent name is then backed by a real session, and
    /// per-role authorization (terminal allowlists, tool gating) can key on
    /// the validated identity. When absent, the call runs under the
    /// documented localhost posture with `identity_validated: false`.
    #[serde(default)]
    pub session_token: Option<String>,
}

/// The normalized result of one McpCall — success, payload, and the real
/// metered usage entry for the call (never fabricated).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpResult {
    pub success: bool,
    #[serde(default)]
    pub output: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
    /// agent, tool, day, total_today, limit, allowed — captured by the
    /// ledger during dispatch.
    pub usage: serde_json::Value,
}

impl McpRegistry {
    /// Dispatch one abstract call: gate on the usage ledger, route by
    /// transport, capture monitor activity. HTTP/SSE tools receive the
    /// standardized envelope {tool, agent, input, context}; stdio tools are
    /// long-lived agents' own processes and are invoked by THOSE agents
    /// directly — the registry still meters and records the call here.
    pub async fn invoke(
        &self,
        call: McpCall,
        usage: &UsageLedger,
        activity: Option<&crate::monitor::ActivityHub>,
    ) -> McpResult {
        self.invoke_with_identity(call, usage, activity, McpCallerIdentity::default())
            .await
    }

    /// Same as `invoke`, but forwards the caller's VALIDATED identity to the
    /// tool in the envelope (`identity_validated`, `session_user_id`) so a
    /// tool server can key its own per-role authorization on a real session
    /// instead of the caller-claimed `agent` string. `invoke` is this with
    /// the unvalidated default, so existing callers are unchanged.
    pub async fn invoke_with_identity(
        &self,
        call: McpCall,
        usage: &UsageLedger,
        activity: Option<&crate::monitor::ActivityHub>,
        identity: McpCallerIdentity,
    ) -> McpResult {
        // NOTE: `agent` is caller-claimed. The usage ledger meters per claimed
        // name, so it is accounting by convention, not an authenticated quota
        // (a caller can rotate names). Validated identity travels separately
        // in `identity` and is forwarded to the tool in the envelope.
        let (allowed, total_today, limit) = usage.record(&call.agent, &call.tool).await;
        // Role authorization trust (2026-11-01): the ledger and the tool
        // envelope both say whether `agent` is BACKED by a validated session
        // or merely DECLARED by the caller. Tool servers that gate by role
        // (terminal allowlists) must treat `agent_trust: "declared"` as an
        // unverified claim and key privileged roles on `"validated"` only.
        let agent_trust = if identity.validated { "validated" } else { "declared" };
        let usage_json = serde_json::json!({
            "agent": call.agent,
            "tool": call.tool,
            "total_today": total_today,
            "daily_limit": limit,
            "allowed": allowed,
            "identity_validated": identity.validated,
            "agent_trust": agent_trust,
        });

        if !allowed {
            return McpResult {
                success: false,
                output: None,
                error: Some(format!(
                    "MCP daily usage limit reached for {} ({}/{} calls today)",
                    call.agent, total_today, limit
                )),
                usage: usage_json,
            };
        }

        let tool = match self.get(&call.tool).await {
            Some(t) => t,
            None => {
                return McpResult {
                    success: false,
                    output: None,
                    error: Some(format!("tool '{}' not registered", call.tool)),
                    usage: usage_json,
                }
            }
        };

        let result = match tool.transport {
            McpTransport::Http | McpTransport::Sse => {
                let envelope = serde_json::json!({
                    "tool": call.tool,
                    "agent": call.agent,
                    "input": call.input,
                    "context": call.context,
                    "identity_validated": identity.validated,
                    "session_user_id": identity.session_user_id,
                    // Role lookups in a tool server must key on this, not on
                    // `agent` alone: "validated" = backed by a real session
                    // (and `session_user_id`); "declared" = the caller's own
                    // claim, the documented localhost posture.
                    "agent_declared": call.agent,
                    "agent_trust": agent_trust,
                    "trace": crate::trace::current_fields(),
                });
                let client = reqwest::Client::new();
                // Host-internal token so MCP servers can refuse calls that did not come
                // through the host (tools/shared/guard.mjs; enforced on strict hosts or with
                // OZONE_MCP_REQUIRE_INTERNAL=1).
                let mut req_builder = client.post(&tool.endpoint);
                if let Some(tok) = crate::grpc::internal_token() {
                    req_builder = req_builder.header("x-oz-internal", tok);
                }
                let resp = req_builder
                    .json(&envelope)
                    .timeout(std::time::Duration::from_secs(30))
                    .send()
                    .await;
                match resp {
                    Ok(r) if r.status().is_success() => match r.json::<serde_json::Value>().await {
                        Ok(mut out) => {
                            // Tools may answer in the standardized shape or
                            // bare — normalize bare payloads on the way out.
                            if !(out.get("success").is_some() || out.get("error").is_some()) {
                                out = serde_json::json!({ "success": true, "output": out });
                            }
                            Ok(out)
                        }
                        Err(e) => Err(format!("unreadable response: {}", e)),
                    },
                    Ok(r) => Err(format!("tool returned HTTP {}", r.status())),
                    Err(e) => Err(format!("tool unreachable: {}", e)),
                }
            }
            McpTransport::Agent => {
                // The provider is a participating agent: create (or adopt) the durable request and await its report.
                // No timeout and no fail-open — the step waits like any gate that awaits (src/agent_requests.rs).
                match crate::agent_requests::request_and_await(
                    &call.tool,
                    &tool.endpoint,
                    &call.agent,
                    &call.input,
                    call.context.as_ref(),
                )
                .await
                {
                    Ok(v) => Ok(if v.get("success").is_some() || v.get("error").is_some() {
                        v
                    } else {
                        serde_json::json!({ "success": true, "output": v })
                    }),
                    Err(e) => Err(e),
                }
            }
            McpTransport::Stdio => {
                // Stdio tools belong to a live agent session — invocation is
                // that agent issuing the call on its own transport. Metered
                // and recorded here; dispatch returns the delegation.
                Ok(serde_json::json!({
                    "success": true,
                    "delegated": true,
                    "tool": call.tool,
                    "message": "stdio tool — invocation delegated to the owning agent session",
                }))
            }
        };

        if let Some(hub) = activity {
            let (level, msg) = match &result {
                Ok(_v) => (
                    crate::monitor::ActivityLevel::Info,
                    format!("{} called tool {} — ok", call.agent, call.tool),
                ),
                Err(e) => (
                    crate::monitor::ActivityLevel::Warn,
                    format!("{} called tool {} — failed: {}", call.agent, call.tool, e),
                ),
            };
            hub.record(crate::monitor::ActivityKind::Tool, level, &call.agent, msg, None);
        }

        match result {
            Ok(v) => {
                let success = v.get("success").and_then(|s| s.as_bool()).unwrap_or(true);
                let error = v.get("error").and_then(|e| e.as_str()).map(String::from);
                McpResult {
                    success,
                    output: Some(v),
                    error,
                    usage: usage_json,
                }
            }
            Err(e) => McpResult {
                success: false,
                output: None,
                error: Some(e),
                usage: usage_json,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // T-U1: per-tool counting, same-day accumulation.
    #[test]
    fn ledger_counts_per_tool() {
        let ledger = UsageLedger::with_limit(0); // unlimited
        let (allowed, total, limit) = smol_block(ledger.record("zcode", "presence_heartbeat"));
        assert!(allowed);
        assert_eq!(total, 1);
        assert_eq!(limit, 0);
        let (_, total, _) = smol_block(ledger.record("zcode", "presence_heartbeat"));
        assert_eq!(total, 2);
        let (_, total, _) = smol_block(ledger.record("zcode", "context_summary"));
        assert_eq!(total, 3);
        let snap = smol_block(ledger.snapshot("zcode"));
        assert_eq!(snap["total_today"], 3);
        // per_tool sorted by count desc: presence_heartbeat(2) then context_summary(1)
        let tools: Vec<(String, u64)> = snap["per_tool"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (p[0].as_str().unwrap().into(), p[1].as_u64().unwrap()))
            .collect();
        assert!(tools.contains(&("presence_heartbeat".into(), 2)));
        assert!(tools.contains(&("context_summary".into(), 1)));
    }

    // T-U1: day rollover — the real boundary, not a fixture guess. One
    // second before midnight UTC and one second after must land on
    // different date strings; the ledger's isolation on (agent, day) then
    // means yesterday's count genuinely doesn't carry over.
    #[test]
    fn day_rollover_boundary_is_correct() {
        // 2026-01-01T23:59:59Z and the following second, 2026-01-02T00:00:00Z.
        let before = 1767311999u64;
        let after = 1767312000u64;
        assert_eq!(utc_date_from_secs(before), "2026-01-01");
        assert_eq!(utc_date_from_secs(after), "2026-01-02");
    }

    // T-U2: gate refuses over limit, still counts the refused call.
    #[test]
    fn ledger_gate_refuses_over_limit() {
        let ledger = UsageLedger::with_limit(2);
        let (allowed, _, _) = smol_block(ledger.record("cc", "tool_a"));
        assert!(allowed);
        let (allowed, _, _) = smol_block(ledger.record("cc", "tool_b"));
        assert!(allowed);
        let (allowed, total, limit) = smol_block(ledger.record("cc", "tool_c"));
        assert!(!allowed, "third call over limit 2 must refuse");
        assert_eq!(total, 3); // refused calls still counted (honest ledger)
        assert_eq!(limit, 2);
    }

    // T-U1: agents are isolated.
    #[test]
    fn ledger_isolates_agents() {
        let ledger = UsageLedger::with_limit(1);
        let (allowed, _, _) = smol_block(ledger.record("a", "t"));
        assert!(allowed);
        let (allowed, _, _) = smol_block(ledger.record("b", "t"));
        assert!(allowed, "agent b has its own budget");
        let (allowed, _, _) = smol_block(ledger.record("a", "t"));
        assert!(!allowed, "agent a hit its limit");
    }

    // T-G1/T-U1 combined: stdio delegation path — metered, delegated, no network.
    #[tokio::test]
    async fn invoke_stdio_tool_delegates_and_meters() {
        let registry = McpRegistry::new();
        registry
            .register(McpTool {
                name: "shared-context".into(),
                transport: McpTransport::Stdio,
                endpoint: "node server.js".into(),
                capabilities: vec![],
                server_version: None,
                registered_at: 0,
                auto_start: false,
            })
            .await;
        let usage = UsageLedger::with_limit(0);
        let result = registry
            .invoke(
                McpCall {
                    session_token: None,
                    tool: "shared-context".into(),
                    agent: "zcode".into(),
                    input: serde_json::json!({"action": "context_summary"}),
                    context: None,
                },
                &usage,
                None,
            )
            .await;
        assert!(result.success);
        assert_eq!(result.usage["total_today"], 1);
        let out = result.output.unwrap();
        assert_eq!(out["delegated"], true);
    }

    #[tokio::test]
    async fn invoke_unknown_tool_fails_with_usage_recorded() {
        let registry = McpRegistry::new();
        let usage = UsageLedger::with_limit(0);
        let result = registry
            .invoke(
                McpCall { tool: "nope".into(), agent: "zcode".into(), input: serde_json::json!({}), context: None, session_token: None },
                &usage,
                None,
            )
            .await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("not registered"));
        assert_eq!(result.usage["total_today"], 1);
    }

    /// Block on a future with a short sleep-based executor (tests run inside
    /// tokio via #[tokio::test]; this helper just awaits directly).
    fn smol_block<T>(fut: impl std::future::Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(fut)
    }
}

#[cfg(test)]
mod global_tests {
    use super::*;

    // T-U5: the process-global path — install, register, invoke via
    // call_global; metered through the same ledger.
    #[tokio::test]
    async fn call_global_delegates_and_meters() {
        let registry = Arc::new(McpRegistry::new());
        let usage = Arc::new(UsageLedger::with_limit(0));
        install_global(registry.clone(), usage.clone());
        registry
            .register(McpTool {
                name: "global-tool".into(),
                transport: McpTransport::Stdio,
                endpoint: "test".into(),
                capabilities: vec![],
                server_version: None,
                registered_at: 0,
                auto_start: false,
            })
            .await;

        let result = call_global(McpCall {
            tool: "global-tool".into(),
            agent: "test-agent".into(),
            input: serde_json::json!({}),
            context: None,
            session_token: None,
        })
        .await;

        assert!(result.success);
        assert_eq!(result.usage["total_today"], 1);
        assert_eq!(result.usage["agent"], "test-agent");
    }
}
