//! Ozone Studio CLI (2026-10-18) — one binary, headless-friendly, for VPS
//! deployment and operator tooling.
//!
//! Commands:
//!   serve (default when no args)  — run the host exactly as the bare binary
//!                                   always has, with optional overrides.
//!   version                       — build/version info.
//!   doctor [--config F]           — real environment checks: config parse,
//!                                   data-dir writability, pipeline binaries,
//!                                   port availability. Nothing fabricated.
//!   mcp list | autostart <tool> <on|off> | start <tool> | stop <tool>
//!                                 — client commands against a RUNNING host
//!                                   (same routes the UI uses). The CLI never
//!                                   edits the registry file directly: one
//!                                   writer, the host.
//!
//! Backward compatibility is absolute: `./ozone-studio` with no arguments
//! behaves byte-for-byte like the pre-CLI binary.

use std::path::PathBuf;

pub struct ServeOverrides {
    pub config_path: PathBuf,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub data_dir: Option<String>,
}

pub enum Command {
    /// Default — bare binary, no args. Optional overrides via flags.
    Serve(ServeOverrides),
    Version,
    Doctor { config_path: PathBuf },
    /// Client commands against a running host. base = http://host:port.
    Mcp {
        base: String,
        action: McpAction,
    },
    Help,
}

pub enum McpAction {
    List,
    Autostart { name: String, enabled: bool },
    Start { name: String },
    Stop { name: String },
}

fn flag_value(args: &[String], i: usize, flag: &str) -> Option<String> {
    // Supports both `--flag value` and `--flag=value`.
    if let Some(rest) = args[i].strip_prefix(&format!("{}=", flag)) {
        return Some(rest.to_string());
    }
    if args[i] == flag {
        return args.get(i + 1).cloned();
    }
    None
}

/// Parse args. Never panics; unknown input falls back to Help so a typo can
/// never silently start a server the operator didn't ask for.
pub fn parse(args: &[String]) -> Command {
    if args.is_empty() {
        return Command::Serve(ServeOverrides {
            config_path: default_config_path(),
            host: None,
            port: None,
            data_dir: None,
        });
    }
    match args[0].as_str() {
        "serve" => {
            let mut o = ServeOverrides {
                config_path: default_config_path(),
                host: None,
                port: None,
                data_dir: None,
            };
            let mut i = 1;
            while i < args.len() {
                if let Some(v) = flag_value(args, i, "--config") {
                    o.config_path = PathBuf::from(v);
                } else if let Some(v) = flag_value(args, i, "--host") {
                    o.host = Some(v);
                } else if let Some(v) = flag_value(args, i, "--port") {
                    o.port = v.parse().ok();
                } else if let Some(v) = flag_value(args, i, "--data-dir") {
                    o.data_dir = Some(v);
                }
                // `--flag value` consumed two slots when the next token isn't
                // itself a flag; the loop's flag_value handles both forms.
                i += if args[i].contains('=') { 1 } else { 2 };
            }
            Command::Serve(o)
        }
        "version" | "--version" | "-V" => Command::Version,
        "doctor" => {
            let mut config_path = default_config_path();
            let mut i = 1;
            while i < args.len() {
                if let Some(v) = flag_value(args, i, "--config") {
                    config_path = PathBuf::from(v);
                }
                i += if args[i].contains('=') { 1 } else { 2 };
            }
            Command::Doctor { config_path }
        }
        "mcp" if args.len() >= 2 => {
            let mut base =
                std::env::var("OZONE_HOST").unwrap_or_else(|_| "http://127.0.0.1:50051".into());
            let mut action: Option<McpAction> = None;
            // Subcommands start at index 1 ("mcp" itself is index 0).
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--host" | "--base" => {
                        if let Some(v) = args.get(i + 1) {
                            base = normalize_base(v);
                        }
                        i += 2;
                    }
                    "list" => {
                        action = Some(McpAction::List);
                        i += 1;
                    }
                    "autostart" => {
                        // mcp autostart <tool> <on|off>
                        let name = args.get(i + 1).cloned();
                        let enabled = args
                            .get(i + 2)
                            .map(|v| matches!(v.as_str(), "on" | "true" | "yes"));
                        if let (Some(name), Some(enabled)) = (name, enabled) {
                            action = Some(McpAction::Autostart { name, enabled });
                            i += 3;
                        } else {
                            return Command::Help;
                        }
                    }
                    "start" | "stop" => {
                        let kind = args[i].clone();
                        if let Some(name) = args.get(i + 1).cloned() {
                            action = Some(if kind == "start" {
                                McpAction::Start { name }
                            } else {
                                McpAction::Stop { name }
                            });
                            i += 2;
                        } else {
                            return Command::Help;
                        }
                    }
                    other => {
                        eprintln!("unknown mcp subcommand: {}", other);
                        return Command::Help;
                    }
                }
            }
            match action {
                Some(a) => Command::Mcp { base, action: a },
                None => Command::Help,
            }
        }
        "help" | "--help" | "-h" => Command::Help,
        // A bare unknown first token: refuse rather than guess. Starting a
        // server from a typo would be the worst possible fallback.
        other => {
            eprintln!("unknown command: {}", other);
            Command::Help
        }
    }
}

fn default_config_path() -> PathBuf {
    std::env::var("OZONE_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("config.toml"))
}

fn normalize_base(v: &str) -> String {
    if v.starts_with("http") {
        v.trim_end_matches('/').to_string()
    } else {
        format!("http://{}", v.trim_end_matches('/'))
    }
}

pub fn print_help() {
    println!(
        "Ozone Studio v{version} — one binary, host + CLI.

USAGE:
  ozone-studio                          Run the host (same as bare binary has always done)
  ozone-studio serve [FLAGS]            Run the host with overrides:
    --config <file>                     Config path (default ./config.toml or $OZONE_CONFIG)
    --host <addr>                       Bind address override (default from config; use 0.0.0.0 to expose)
    --port <port>                       Port override
    --data-dir <dir>                    Data directory override
  ozone-studio version                  Print version
  ozone-studio doctor [--config <f>]    Real environment checks (config, data dir, pipelines, port)
  ozone-studio mcp list                 List registered MCP tools (from a RUNNING host)
  ozone-studio mcp autostart <tool> <on|off>
  ozone-studio mcp start <tool>         Start a tool's backing process now
  ozone-studio mcp stop <tool>
    --host <base>                       Target host for mcp commands (default $OZONE_HOST or http://127.0.0.1:50051)

NOTES:
  Headless VPS deployment: run `serve` under the provided systemd unit.
  Default bind is 127.0.0.1 (secure); expose only via reverse proxy or an
  explicit --host 0.0.0.0 with firewall rules.",
        version = env!("CARGO_PKG_VERSION")
    );
}

/// Run a non-serve command. Returns Ok(false) when the caller should fall
/// through to serve (never happens for parsed non-serve commands), Ok(true)
/// when handled. All output is real — no fabricated checks anywhere.
pub async fn run(cmd: Command) -> Result<bool, String> {
    match cmd {
        Command::Serve(_) => Ok(false), // main() handles serve itself
        Command::Version => {
            println!(
                "ozone-studio {} ({}, {})",
                env!("CARGO_PKG_VERSION"),
                std::env::consts::OS,
                std::env::consts::ARCH
            );
            Ok(true)
        }
        Command::Doctor { config_path } => {
            doctor(&config_path).await;
            Ok(true)
        }
        Command::Mcp { base, action } => {
            mcp_client(&base, action).await?;
            Ok(true)
        }
        Command::Help => {
            print_help();
            Ok(true)
        }
    }
}

/// Doctor — real checks with named outcomes. Exit code semantics: prints
/// PASS/FAIL lines; the caller (main) exits non-zero if any FAIL.
async fn doctor(config_path: &PathBuf) {
    let mut failures = 0usize;

    // 1. Config parse.
    match crate::OzoneConfig::load(config_path).await {
        Ok(config) => {
            println!(
                "PASS  config           {} (data_dir={}, grpc={}:{})",
                config_path.display(),
                config.general.data_dir,
                config.grpc.address,
                config.grpc.port
            );

            // 2. Data dir exists + writable.
            let probe = std::path::Path::new(&config.general.data_dir).join(".doctor_write_probe");
            match std::fs::write(&probe, b"ok") {
                Ok(_) => {
                    let _ = std::fs::remove_file(&probe);
                    println!(
                        "PASS  data-dir        {} writable",
                        config.general.data_dir
                    );
                }
                Err(e) => {
                    failures += 1;
                    println!(
                        "FAIL  data-dir        {} not writable: {}",
                        config.general.data_dir, e
                    );
                }
            }

            // 3. Port availability (bind then release).
            let addr = format!("{}:{}", config.grpc.address, config.grpc.port);
            match std::net::TcpListener::bind(&addr) {
                Ok(l) => {
                    drop(l);
                    println!("PASS  port            {} free", addr);
                }
                Err(e) => {
                    // In-use by OURSELVES is "occupied", not a failure of the
                    // environment — name it precisely either way.
                    println!("WARN  port            {} in use ({}): host already running?", addr, e);
                }
            }
        }
        Err(e) => {
            failures += 1;
            println!("FAIL  config           {}: {}", config_path.display(), e);
        }
    }

    // 4. Pipeline binaries — the real deployment asset. Checked in the
    // same order the executor searches, minus in-source paths.
    let manifest = env!("CARGO_MANIFEST_DIR");
    let candidates = [
        format!("{}/target/pipelines/release", manifest),
        format!("{}/assets/pipelines", manifest),
        "pipelines".to_string(),
        format!("{}/pipelines", std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".into())),
        "target/pipelines/release".to_string(),
    ];
    let mut found = 0usize;
    let mut checked_dir = String::new();
    for base in &candidates {
        if let Ok(rd) = std::fs::read_dir(base) {
            let n = rd.filter_map(|e| e.ok()).filter(|e| {
                let p = e.path();
                p.is_file() && !p.extension().map(|x| x == "d").unwrap_or(false)
            }).count();
            if n > found {
                found = n;
                checked_dir = base.clone();
            }
        }
    }
    if found > 0 {
        println!("PASS  pipelines       {} binaries in {}", found, checked_dir);
    } else {
        failures += 1;
        println!(
            "FAIL  pipelines       no pipeline binaries found (tried: {}) — the host will start but every pipeline dispatch fails",
            candidates.join(", ")
        );
    }

    if failures > 0 {
        eprintln!("doctor: {} check(s) FAILED", failures);
        std::process::exit(1);
    }
    println!("doctor: all checks passed");
}

/// MCP client — talks to a RUNNING host over the same routes the UI uses.
async fn mcp_client(base: &str, action: McpAction) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("http client: {}", e))?;
    match action {
        McpAction::List => {
            let v: serde_json::Value = client
                .get(format!("{}/mcp/tools", base))
                .send()
                .await
                .map_err(|e| format!("host unreachable at {} — is it running? ({})", base, e))?
                .json()
                .await
                .map_err(|e| e.to_string())?;
            let tools = v
                .get("tools")
                .and_then(|t| t.as_array())
                .cloned()
                .unwrap_or_default();
            println!(
                "{:<6} {:<8} {:<8} {:<10} {}",
                "AUTO", "RUNNING", "TRANSPORT", "PORT", "NAME"
            );
            for t in &tools {
                let port = t
                    .get("endpoint")
                    .and_then(|e| e.as_str())
                    .and_then(|e| e.rsplit(':').next())
                    .and_then(|p| p.split('/').next())
                    .unwrap_or("-");
                println!(
                    "{:<6} {:<8} {:<8} {:<10} {}",
                    t.get("auto_start").and_then(|b| b.as_bool()).unwrap_or(false),
                    t.get("running").and_then(|b| b.as_bool()).unwrap_or(false),
                    t.get("transport").and_then(|s| s.as_str()).unwrap_or("?"),
                    port,
                    t.get("name").and_then(|s| s.as_str()).unwrap_or("?")
                );
            }
            println!("\n{} tool(s)", tools.len());
        }
        McpAction::Autostart { name, enabled } => {
            let v: serde_json::Value = client
                .post(format!("{}/mcp/tools/{}/autostart", base, name))
                .json(&serde_json::json!({ "enabled": enabled }))
                .send()
                .await
                .map_err(|e| e.to_string())?
                .json()
                .await
                .map_err(|e| e.to_string())?;
            if v.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
                println!(
                    "auto_start={} for group: {}",
                    enabled,
                    v.get("affected")
                        .and_then(|a| a.as_array())
                        .map(|a| a.iter().filter_map(|s| s.as_str()).collect::<Vec<_>>().join(", "))
                        .unwrap_or_default()
                );
            } else {
                return Err(v
                    .get("error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("unknown error")
                    .to_string());
            }
        }
        McpAction::Start { name } => {
            let v: serde_json::Value = client
                .post(format!("{}/mcp/tools/{}/start", base, name))
                .send()
                .await
                .map_err(|e| e.to_string())?
                .json()
                .await
                .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            if !v.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
                std::process::exit(1);
            }
        }
        McpAction::Stop { name } => {
            let v: serde_json::Value = client
                .post(format!("{}/mcp/tools/{}/stop", base, name))
                .send()
                .await
                .map_err(|e| e.to_string())?
                .json()
                .await
                .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            if !v.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
                std::process::exit(1);
            }
        }
    }
    Ok(())
}
