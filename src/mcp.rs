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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpTransport {
    Stdio,
    Http,
    Sse,
}

impl McpTransport {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "stdio" => Some(Self::Stdio),
            "http" => Some(Self::Http),
            "sse" => Some(Self::Sse),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stdio => "stdio",
            Self::Http => "http",
            Self::Sse => "sse",
        }
    }
}

/// One registered tool.
#[derive(Debug, Clone)]
pub struct McpTool {
    pub name: String,
    pub transport: McpTransport,
    /// Command+args (stdio) or URL (http/sse).
    pub endpoint: String,
    pub capabilities: Vec<String>,
    pub server_version: Option<String>,
    pub registered_at: u64,
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

/// Issue the standardized abstract call through the process-global handles.
/// Same metering, gating, and rippling as the HTTP path.
pub async fn call_global(call: McpCall) -> McpResult {
    let (registry, usage) = GLOBAL_MCP
        .get()
        .expect("mcp global handles not installed — call install_global at startup");
    registry.invoke(call, usage, None).await
}

/// The registry itself — one per host, shared through AppState.
#[derive(Default)]
pub struct McpRegistry {
    tools: RwLock<HashMap<String, McpTool>>,
}

impl McpRegistry {
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
        }
    }

    /// Register (or idempotently refresh) a tool by name.
    pub async fn register(&self, tool: McpTool) -> bool {
        let replaced = self.tools.read().await.contains_key(&tool.name);
        self.tools.write().await.insert(tool.name.clone(), tool);
        replaced
    }

    pub async fn unregister(&self, name: &str) -> bool {
        self.tools.write().await.remove(name).is_some()
    }

    pub async fn get(&self, name: &str) -> Option<McpTool> {
        self.tools.read().await.get(name).cloned()
    }

    pub async fn list(&self) -> Vec<McpTool> {
        let mut tools: Vec<McpTool> = self.tools.read().await.values().cloned().collect();
        tools.sort_by(|a, b| a.name.cmp(&b.name));
        tools
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
        let (allowed, total_today, limit) = usage.record(&call.agent, &call.tool).await;
        let usage_json = serde_json::json!({
            "agent": call.agent,
            "tool": call.tool,
            "total_today": total_today,
            "daily_limit": limit,
            "allowed": allowed,
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
                });
                let client = reqwest::Client::new();
                let resp = client
                    .post(&tool.endpoint)
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
                Ok(v) => (
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
            })
            .await;
        let usage = UsageLedger::with_limit(0);
        let result = registry
            .invoke(
                McpCall {
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
                McpCall { tool: "nope".into(), agent: "zcode".into(), input: serde_json::json!({}), context: None },
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
            })
            .await;

        let result = call_global(McpCall {
            tool: "global-tool".into(),
            agent: "test-agent".into(),
            input: serde_json::json!({}),
            context: None,
        })
        .await;

        assert!(result.success);
        assert_eq!(result.usage["total_today"], 1);
        assert_eq!(result.usage["agent"], "test-agent");
    }
}
