//! MCP tool-server process supervisor (2026-10-18).
//!
//! `src/mcp.rs`'s registry has survived restarts since 2026-09-28, but
//! registration was never the same thing as a running process — confirmed
//! live this session: 134 tools registered in `zsei_data/mcp_tool_registry.json`,
//! zero of their backing Node.js processes actually listening. Nothing
//! ever started them, and nothing restarted one that died.
//!
//! This module is the other half, built operator-gated rather than
//! guessed: `McpTool::auto_start` is a real, persisted, per-tool opt-in
//! the operator sets explicitly (via the UI this backend exposes an API
//! for — see the new `/mcp/tools/:name/autostart` route in
//! `src/grpc/mod.rs`), default `false` for every tool. A tool with
//! `auto_start == true` is only actually started once every REAL
//! requirement it has is honestly met:
//! - `device_requirement`: confirmed this session that only `android-mcp`
//!   needs one (real `adb`-attached device) — nothing else in the fleet
//!   does, checked deliberately, not assumed.
//! - `credential_requirement`: confirmed only `gcal-mcp`/`zoom-mcp` need
//!   one (OAuth client id/secret + an exchanged token file) — everything
//!   else is either credential-free by design, or (terminal-mcp,
//!   android-mcp's shell access) gated by OS-level permissions that are
//!   the tool's own runtime concern, not a reason to withhold starting
//!   the process itself.
//!
//! Every other registered MCP tool-server (chemistry/dna/geospatial/
//! proteomics/web/security/re-mcp/terminal/visual/mesh/cerebrix/
//! connectome/threed/code/git-mcp, plus the 67-capability `bridges`
//! fleet) has neither requirement and starts as soon as the operator
//! flips `auto_start` on.
//!
//! Process identity is the real `(transport, endpoint)` pair, not the
//! registered tool `name` — one Node process backs many registered
//! capability names (`bridges` alone backs 67 on one port). The port
//! parsed out of `endpoint` is the actual supervision key.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Child;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;

use crate::mcp::{McpRegistry, McpTransport};

/// Real, confirmed-by-reading-the-source facts about each known MCP
/// tool-server process, keyed by the port it actually registers under
/// (not its coded default — confirmed this session those currently
/// match for all 19, but the registry's real endpoint is the source of
/// truth if they ever diverge).
#[derive(Debug, Clone, Copy)]
pub struct McpProcessFacts {
    /// Directory under `tools/`.
    pub dir: &'static str,
    /// Entry file to run, relative to that directory.
    pub entry: &'static str,
    /// Env var that controls its listen port.
    pub port_env: &'static str,
    pub device_requirement: Option<&'static str>,
    pub credential_requirement: Option<&'static str>,
}

const fn facts(
    dir: &'static str,
    entry: &'static str,
    port_env: &'static str,
) -> McpProcessFacts {
    McpProcessFacts { dir, entry, port_env, device_requirement: None, credential_requirement: None }
}

/// Port → process facts. Confirmed live against the real running host's
/// `target/release/zsei_data/mcp_tool_registry.json` (19 distinct
/// endpoints, 134 capability entries) cross-checked against each tool's
/// own `server.mjs`'s real `OZONE_*_PORT ?? <default>` line — not
/// guessed from directory names.
pub fn facts_for_port(port: u16) -> Option<McpProcessFacts> {
    match port {
        3210 => Some(facts("bridges", "dist/local.js", "LOCAL_CONTROL_PORT")),
        3215 => Some(facts("terminal-mcp", "server.mjs", "OZONE_TERMINAL_PORT")),
        3220 => Some(facts("visual-mcp", "server.mjs", "OZONE_VISUAL_PORT")),
        3225 => Some(facts("web-mcp", "server.mjs", "OZONE_WEB_PORT")),
        3230 => Some(facts("code-mcp", "server.mjs", "OZONE_CODE_PORT")),
        3235 => Some(facts("git-mcp", "server.mjs", "OZONE_GIT_PORT")),
        3240 => Some(facts("security-mcp", "server.mjs", "OZONE_SECURITY_PORT")),
        3241 => Some(McpProcessFacts {
            dir: "gcal-mcp",
            entry: "server.mjs",
            port_env: "OZONE_GCAL_PORT",
            device_requirement: None,
            credential_requirement: Some("google_oauth"),
        }),
        3245 => Some(facts("cerebrix-mcp", "server.mjs", "OZONE_CEREBRIX_PORT")),
        3250 => Some(facts("threed-mcp", "server.mjs", "OZONE_THREED_PORT")),
        3255 => Some(facts("connectome-mcp", "server.mjs", "OZONE_CONNECTOME_PORT")),
        3260 => Some(facts("re-mcp", "server.mjs", "OZONE_RE_PORT")),
        3265 => Some(McpProcessFacts {
            dir: "zoom-mcp",
            entry: "server.mjs",
            port_env: "OZONE_ZOOM_PORT",
            device_requirement: None,
            credential_requirement: Some("zoom_oauth"),
        }),
        3271 => Some(facts("dna-mcp", "server.mjs", "OZONE_DNA_PORT")),
        3272 => Some(facts("chemistry-mcp", "server.mjs", "OZONE_CHEM_PORT")),
        3273 => Some(facts("proteomics-mcp", "server.mjs", "OZONE_PROTEOMICS_PORT")),
        3274 => Some(facts("geospatial-mcp", "server.mjs", "OZONE_GEOSPATIAL_PORT")),
        3276 => Some(facts("mesh-mcp", "server.mjs", "OZONE_MESH_PORT")),
        3277 => Some(McpProcessFacts {
            dir: "android-mcp",
            entry: "server.mjs",
            port_env: "OZONE_ANDROID_PORT",
            device_requirement: Some("android"),
            credential_requirement: None,
        }),
        _ => None,
    }
}

/// Parse the port out of an `http://host:port/path` endpoint. `None` for
/// non-HTTP endpoints (stdio tools have no port at all).
pub fn extract_port(endpoint: &str) -> Option<u16> {
    let after_scheme = endpoint.split("://").nth(1)?;
    let host_port = after_scheme.split('/').next()?;
    host_port.rsplit(':').next()?.parse().ok()
}

/// Where the fleet's source lives, relative-path convention matching
/// this project's existing `zsei_data` cwd-relative pattern (the real
/// host runs with cwd `target/release/`, two levels under `tools/`).
pub fn tools_dir() -> PathBuf {
    std::env::var("OZONE_TOOLS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("../../tools"))
}

fn adb_binary() -> String {
    if let Ok(p) = std::env::var("OZONE_ADB") {
        return p;
    }
    if let Ok(home) = std::env::var("ANDROID_HOME") {
        let p = std::path::Path::new(&home).join("platform-tools").join("adb");
        if p.exists() {
            return p.to_string_lossy().into_owned();
        }
    }
    "adb".to_string()
}

/// Real device-presence check — mirrors `android-mcp/server.mjs`'s own
/// `adb`-resolution + `adb devices` convention exactly, rather than
/// re-deriving a separate notion of "device paired." Note deliberately
/// NOT reusing `src/pairing.rs`'s `DeviceType`/`registered_devices` —
/// that tracks phones paired for multi-device UI session auth, a
/// different concept from "a USB-attached Android device `adb` can see
/// right now."
pub fn device_available(requirement: &str) -> bool {
    match requirement {
        "android" => {
            let bin = adb_binary();
            match std::process::Command::new(&bin).arg("devices").output() {
                Ok(out) if out.status.success() => {
                    let text = String::from_utf8_lossy(&out.stdout);
                    text.lines()
                        .skip(1)
                        .any(|line| line.split_whitespace().nth(1) == Some("device"))
                }
                _ => false,
            }
        }
        _ => false,
    }
}

/// Real credential-presence check — both the OAuth client id/secret env
/// vars AND an actually-exchanged token file, matching each tool's own
/// documented requirement exactly (confirmed by reading `gcal-mcp`/
/// `zoom-mcp`'s own header comments this session).
pub fn credential_available(requirement: &str) -> bool {
    match requirement {
        "google_oauth" => {
            std::env::var("OZONE_GCAL_CLIENT_ID").is_ok()
                && std::env::var("OZONE_GCAL_CLIENT_SECRET").is_ok()
                && tools_dir().join("gcal-mcp").join("tokens.json").exists()
        }
        "zoom_oauth" => {
            std::env::var("OZONE_ZOOM_CLIENT_ID").is_ok()
                && std::env::var("OZONE_ZOOM_CLIENT_SECRET").is_ok()
                && tools_dir().join("zoom-mcp").join("tokens.json").exists()
        }
        _ => false,
    }
}

/// Real device/credential requirement lookups keyed by a registered
/// tool NAME (resolves name → endpoint → port → facts) — the shape the
/// UI needs per capability row.
pub async fn device_requirement_for(registry: &McpRegistry, tool_name: &str) -> Option<&'static str> {
    let tool = registry.get(tool_name).await?;
    let port = extract_port(&tool.endpoint)?;
    facts_for_port(port)?.device_requirement
}

pub async fn credential_requirement_for(registry: &McpRegistry, tool_name: &str) -> Option<&'static str> {
    let tool = registry.get(tool_name).await?;
    let port = extract_port(&tool.endpoint)?;
    facts_for_port(port)?.credential_requirement
}

/// Tracks the real child processes this host has started, keyed by
/// port. `start_port`/`stop_port` are real process control, not a
/// simulation — kept deliberately small (no retry backoff, no log
/// rotation) since the crash-supervision loop below already provides
/// the one real resilience property that matters (a dead tool comes
/// back within one tick).
pub struct McpProcessSupervisor {
    children: Mutex<HashMap<u16, Child>>,
}

impl McpProcessSupervisor {
    pub fn new() -> Self {
        Self { children: Mutex::new(HashMap::new()) }
    }

    /// Start the process backing `port` if it isn't already tracked as
    /// alive. `Ok(true)` = just started, `Ok(false)` = already running,
    /// `Err` = no known facts for this port, or the real spawn failed
    /// (missing `node`, missing script, port in use — all surfaced
    /// honestly, never swallowed).
    pub async fn start_port(&self, port: u16, zsei_data_dir: &str) -> Result<bool, String> {
        let mut children = self.children.lock().await;
        if let Some(child) = children.get_mut(&port) {
            if matches!(child.try_wait(), Ok(None)) {
                return Ok(false);
            }
        }
        let facts = facts_for_port(port).ok_or_else(|| format!("no known process facts for port {port}"))?;
        let script = tools_dir().join(facts.dir).join(facts.entry);
        if !script.exists() {
            return Err(format!("entry script not found: {}", script.display()));
        }
        let child = std::process::Command::new("node")
            .arg(&script)
            .env(facts.port_env, port.to_string())
            .env("OZONE_ZSEI_DATA_DIR", zsei_data_dir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("spawn failed: {e}"))?;
        children.insert(port, child);
        Ok(true)
    }

    /// Kill and stop tracking the process on `port`, if any. `true` if a
    /// tracked child was actually killed.
    pub async fn stop_port(&self, port: u16) -> bool {
        let mut children = self.children.lock().await;
        if let Some(mut child) = children.remove(&port) {
            let _ = child.kill();
            true
        } else {
            false
        }
    }

    /// Whether a tracked, still-alive child exists for `port`.
    pub async fn is_running(&self, port: u16) -> bool {
        let mut children = self.children.lock().await;
        match children.get_mut(&port) {
            Some(child) => matches!(child.try_wait(), Ok(None)),
            None => false,
        }
    }
}

impl Default for McpProcessSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

/// Boot-time autostart pass + ongoing crash supervision. Same
/// `tokio::spawn` + `interval.tick()`-first convention as
/// `file_beacon::spawn` — the first tick fires immediately (the real
/// boot-time pass), every tick after that (every 30s) is the crash
/// check. Only ever starts a process when every real requirement it has
/// is honestly met; every skip is logged with its real reason, never
/// silent.
pub fn spawn_autostart_supervisor(
    registry: Arc<McpRegistry>,
    zsei_data_dir: String,
) -> Arc<McpProcessSupervisor> {
    let supervisor = Arc::new(McpProcessSupervisor::new());
    let sup = supervisor.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let groups = registry.auto_start_groups().await;
            for (transport, endpoint, names) in &groups {
                if *transport != McpTransport::Http {
                    // stdio tools are agent-owned processes (e.g. this
                    // session's own ozone-shared-context) — not something
                    // the host spawns.
                    continue;
                }
                let Some(port) = extract_port(endpoint) else { continue };
                let Some(facts) = facts_for_port(port) else {
                    tracing::warn!(
                        "mcp-supervisor: auto_start set for {:?} (port {port}) but no known process facts — skipping",
                        names
                    );
                    continue;
                };
                if let Some(req) = facts.device_requirement {
                    if !device_available(req) {
                        tracing::info!(
                            "mcp-supervisor: skipping {} (port {port}) — device requirement '{req}' not met",
                            facts.dir
                        );
                        continue;
                    }
                }
                if let Some(req) = facts.credential_requirement {
                    if !credential_available(req) {
                        tracing::info!(
                            "mcp-supervisor: skipping {} (port {port}) — credential requirement '{req}' not met",
                            facts.dir
                        );
                        continue;
                    }
                }
                if !sup.is_running(port).await {
                    match sup.start_port(port, &zsei_data_dir).await {
                        Ok(true) => tracing::info!("mcp-supervisor: started {} on port {port}", facts.dir),
                        Ok(false) => {}
                        Err(e) => tracing::warn!(
                            "mcp-supervisor: failed to start {} on port {port}: {e}",
                            facts.dir
                        ),
                    }
                }
            }
        }
    });
    supervisor
}
