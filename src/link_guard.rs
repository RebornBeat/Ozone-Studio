//! Host-side link-path guard (MCP hardening H4, docs/evidence/mcp-participants/REPORT.md).
//!
//! The MCP surface restricts what ITS callers may link, but the host itself accepted any path from a direct HTTP
//! caller (`/orchestrate` `attached_files`, FileLink pipeline 30 via `/pipeline/execute`). This guard is the host's
//! own check, applied at those two entry points:
//!   * an operator allow-list, `OZONE_LINK_ROOTS` (":"-separated). When set, a path must resolve inside one root;
//!   * always (unless `OZONE_LINK_GUARD=off`): paths that hold the host's own secrets or the OS control surface are
//!     off-limits — the data dir, the host config, the internal token, device keys, `~/.ssh`, `~/.gnupg`, `~/.aws`,
//!     `~/.ozone-cli`, `/etc`, `/proc`, `/sys`, `/dev`.
//! It is a security refusal about WHAT a path is, never about size. Symlinks are resolved before the check.

use std::path::{Component, Path, PathBuf};

/// Default ON; only an explicit off value disables it (same convention as every other switch).
pub fn enabled() -> bool {
    !matches!(
        std::env::var("OZONE_LINK_GUARD")
            .ok()
            .map(|v| v.trim().to_ascii_lowercase())
            .as_deref(),
        Some("off") | Some("0") | Some("false") | Some("no")
    )
}

/// Lexically clean an absolute path (no `..`, no `.`) for paths that do not exist yet.
fn clean(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn resolve(p: &str) -> PathBuf {
    let raw = PathBuf::from(p);
    let abs = if raw.is_absolute() {
        raw
    } else {
        std::env::current_dir().map(|d| d.join(&raw)).unwrap_or(raw)
    };
    std::fs::canonicalize(&abs).unwrap_or_else(|_| clean(&abs))
}

fn allow_roots() -> Vec<PathBuf> {
    std::env::var("OZONE_LINK_ROOTS")
        .unwrap_or_default()
        .split(':')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(resolve)
        .collect()
}

fn denied_prefixes() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    v.push(resolve(&data_dir));
    if let Ok(home) = std::env::var("HOME") {
        for sub in [".ssh", ".gnupg", ".aws", ".ozone-cli", ".config/gcloud"] {
            v.push(resolve(&format!("{home}/{sub}")));
        }
    }
    for p in ["/etc", "/proc", "/sys", "/dev"] {
        v.push(PathBuf::from(p));
    }
    v
}

fn denied_name(p: &Path) -> bool {
    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.starts_with("config.toml")
        || name == ".internal_token"
        || name == "device-key.pem"
        || name == "id_rsa"
        || name == "id_ed25519"
}

/// Check one path. `Ok(resolved)` when it may be linked or attached, `Err(reason)` otherwise.
pub fn check(path: &str) -> Result<PathBuf, String> {
    if path.is_empty() {
        return Err("path is empty".to_string());
    }
    if path.contains('\0') {
        return Err("path contains a NUL byte".to_string());
    }
    let real = resolve(path);
    if !enabled() {
        return Ok(real);
    }
    let roots = allow_roots();
    if !roots.is_empty() && !roots.iter().any(|r| real.starts_with(r)) {
        return Err(format!(
            "path is outside the linkable roots (OZONE_LINK_ROOTS); resolved to {}",
            real.display()
        ));
    }
    if denied_name(&real) {
        return Err(format!(
            "{} holds host secrets and cannot be linked or attached",
            real.display()
        ));
    }
    if let Some(p) = denied_prefixes().into_iter().find(|d| real.starts_with(d)) {
        return Err(format!(
            "{} is inside {} (host secrets / OS control surface) and cannot be linked or attached",
            real.display(),
            p.display()
        ));
    }
    Ok(real)
}

/// Check every path-bearing field a link or attach request can carry: `file_path`, `folder_path`, and the
/// `file_path` of each `attached_files` entry. Walks the JSON so a nested shape cannot hide a path.
pub fn check_json(v: &serde_json::Value) -> Result<(), String> {
    match v {
        serde_json::Value::Object(m) => {
            for (k, val) in m {
                if (k == "file_path" || k == "folder_path") && val.is_string() {
                    check(val.as_str().unwrap_or_default())?;
                } else {
                    check_json(val)?;
                }
            }
            Ok(())
        }
        serde_json::Value::Array(a) => a.iter().try_for_each(check_json),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denies_secret_names_and_os_surface() {
        assert!(check("/etc/passwd").is_err());
        assert!(check("/proc/self/environ").is_err());
        assert!(check("/some/where/config.toml").is_err());
        assert!(check("/some/where/device-key.pem").is_err());
        assert!(check("bad\0path").is_err());
    }

    #[test]
    fn ordinary_project_path_is_allowed() {
        assert!(check("/tmp/project/src/main.rs").is_ok());
    }

    #[test]
    fn json_walk_finds_nested_attached_file() {
        let v = serde_json::json!({ "attached_files": [ { "file_path": "/etc/shadow" } ] });
        assert!(check_json(&v).is_err());
        let ok = serde_json::json!({ "data": { "action": "Link", "file_path": "/tmp/a.txt" } });
        assert!(check_json(&ok).is_ok());
    }
}
