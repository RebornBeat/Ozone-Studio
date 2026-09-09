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
use std::sync::Arc;
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
