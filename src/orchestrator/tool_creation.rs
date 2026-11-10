//! TOOL CREATION ON AN IDENTIFIED NEED (operator 2026-11-10) — the capability closure's missing half.
//!
//! Operator ruling: "when we identify a need of a tool and we create one, we publish the findings (like methodologies) — it is based on
//! FINDINGS, not offers, and that is AUTOMATIC. If we don't have the call site to create one for the tool creation on identification of
//! need, such as we do for others, create it." The closure already JUDGES a missing capability (strength-5, `stages::close_capability_gaps`)
//! and drafts a METHODOLOGY for it on the spot; this module is the tool half: ONE zero-shot call site (`capability_tool_draft`, E30)
//! drafts a tool specification, this module validates it by real parsing, writes the artifact into the tool's OWN folder, registers it in
//! the MCP registry, records the finding's provenance on the gap, and hands the finding to the marketplace bridge (a manifest only).
//!
//! WHAT A CREATED TOOL IS, AND WHAT EXECUTES (read this before changing anything — it is the actual risk surface):
//!   * ARTIFACT: `<data_dir>/created_tools/<name>/` — `tool.json` (the one-file body that is hashed and, after a peer's reviewed fetch request,
//!     served: spec + implementation, NEVER the request or the origin), `implementation.mjs` (the drafted Node ES module, a plain file),
//!     `origin.json` (provenance: gap, request, methodology id, task id — local only, never published).
//!   * REGISTRATION: one row in the MCP registry with transport `stdio` and `auto_start = false`. The host NEVER spawns a stdio tool:
//!     `McpRegistry::invoke` answers a stdio call with "delegated to the owning agent session" and `mcp_supervisor` only starts the fixed table
//!     of known tool servers by port. So NOTHING the model wrote is executed by the host — not at creation, not at call time, not at boot.
//!   * It is NOT a custom pipeline: pipeline 15 (`PipelineCreation`) is a stub (Create returns an id and writes nothing), and a real custom
//!     pipeline (`register_custom` + a file under `custom_path`) is run by `executor::invoke_pipeline` as an UNSANDBOXED child process with
//!     the host's privileges and `OZONE_INTERNAL_TOKEN` in its environment — creating one from model text would be arbitrary code execution
//!     at host privilege, so this module does not do that. (A blueprint step also cannot call a custom pipeline: non-provider steps are
//!     coerced to pipeline 9 — `stages.rs`.)
//!   * Running the drafted implementation (an operator or an agent session starting `node implementation.mjs`) is the same explicit,
//!     per-tool operator opt-in every other registered tool needs (`McpTool::auto_start`, "never pre-decided by a registration call").
//!   * The endpoint of the row is the tool's own NAME (a stdio row's endpoint is a command line that is never run here), so
//!     `provider_for_endpoint` yields the name and `provider` / `tool` / `capability` bindings all resolve to it.
//! NO boundary is weakened: nothing is written outside the tool's folder (the name is a validated slug, the folder is created with
//! `create_dir` so an existing one is a collision, files are `create_new`), nothing existing is overwritten, an existing registry name is a
//! collision, and the kill switch `OZONE_TOOL_CREATION=off` turns the whole thing off.

use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Kill switch: `OZONE_TOOL_CREATION=off` (or `0` / `false`) — the closure then behaves as before (methodology only, gap stays recorded).
pub fn enabled() -> bool {
    !matches!(std::env::var("OZONE_TOOL_CREATION").ok().as_deref(), Some("off") | Some("0") | Some("false"))
}

/// The data dir, resolved exactly like `capability_gaps.json` / the marketplace files (`OZONE_ZSEI_DATA_DIR`, default `zsei_data`).
pub fn data_dir() -> PathBuf {
    PathBuf::from(std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string()))
}

/// A tool NAME as a safe slug: lower-case ASCII letters / digits / `_` / `-`, any other run of characters becomes one `-`, no leading or trailing
/// separator, must start with a letter or digit and be 2..=80 characters. This is shape validation (the name becomes a folder name and a
/// registry key), not a limit on content. `None` = not usable as a name.
pub fn sanitize_name(raw: &str) -> Option<String> {
    let mut out = String::new();
    let mut last_sep = false;
    for c in raw.trim().chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
            last_sep = false;
        } else if !last_sep && !out.is_empty() {
            out.push('-');
            last_sep = true;
        }
    }
    let out = out.trim_matches(|c| c == '-' || c == '_').to_string();
    let first_ok = out.chars().next().map(|c| c.is_ascii_alphanumeric()).unwrap_or(false);
    if out.len() < 2 || out.len() > 80 || !first_ok {
        None
    } else {
        Some(out)
    }
}

fn is_placeholder(s: &str) -> bool {
    let t = s.trim();
    t.is_empty() || t == "..." || t == "…"
}

/// A validated tool specification (what the model drafted, after real parsing).
#[derive(Debug, Clone, PartialEq)]
pub struct ToolSpec {
    /// The registry name = the slug of the GAP's name (so a `provider` / `tool` / `capability` binding for that gap resolves to it).
    pub name: String,
    pub description: String,
    /// The drafted capability strings plus the gap's own name (so a free-text capability binding finds it).
    pub capabilities: Vec<String>,
    pub input_contract: Value,
    pub output_contract: Value,
    /// A Node.js ES module that answers the standardized envelope (`{tool, agent, input, context}` → `{success, output}`).
    pub implementation: String,
}

/// Parse the model's draft into a [`ToolSpec`]. REAL parsing: every field is checked by shape — a placeholder shell, a non-object contract, a
/// non-JavaScript language, an implementation with no code line, or a `{"skip": true}` answer is an `Err` (= nothing is created). The tool's
/// name is NOT taken from the model: it is the slug of the gap's name, so the registry row satisfies the binding that reported the gap.
pub fn parse_spec(parsed: &Value, gap_name: &str) -> Result<ToolSpec, String> {
    if parsed.get("skip").and_then(|s| s.as_bool()).unwrap_or(false) {
        return Err("the model judged that no tool can be specified for this capability".to_string());
    }
    let name = sanitize_name(gap_name).ok_or_else(|| format!("the capability name `{gap_name}` is not usable as a tool name"))?;
    let description = parsed.get("description").and_then(|d| d.as_str()).unwrap_or("").trim().to_string();
    if is_placeholder(&description) {
        return Err("no description".to_string());
    }
    let language = parsed.get("language").and_then(|l| l.as_str()).unwrap_or("").trim().to_ascii_lowercase();
    if language != "javascript" {
        return Err(format!("language must be `javascript` (got `{language}`)"));
    }
    let mut capabilities: Vec<String> = parsed
        .get("capabilities")
        .and_then(|c| c.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).map(|s| s.trim().to_string()).filter(|s| !is_placeholder(s)).collect())
        .unwrap_or_default();
    capabilities.push(gap_name.trim().to_string());
    let mut seen = std::collections::HashSet::new();
    capabilities.retain(|c| seen.insert(c.to_lowercase()));
    let object = |key: &str| -> Result<Value, String> {
        match parsed.get(key) {
            Some(v) if v.as_object().map(|o| !o.is_empty()).unwrap_or(false) => Ok(v.clone()),
            _ => Err(format!("`{key}` must be a non-empty JSON object")),
        }
    };
    let (input_contract, output_contract) = (object("input_contract")?, object("output_contract")?);
    let implementation = parsed.get("implementation").and_then(|i| i.as_str()).unwrap_or("").to_string();
    if is_placeholder(&implementation) || implementation.contains('\0') {
        return Err("no implementation".to_string());
    }
    let has_code = implementation.lines().map(|l| l.trim()).any(|l| !l.is_empty() && !l.starts_with("//") && !l.starts_with("/*") && !l.starts_with('*') && l != "...");
    if !has_code {
        return Err("the implementation has no code line".to_string());
    }
    Ok(ToolSpec { name, description, capabilities, input_contract, output_contract, implementation })
}

/// Why this name cannot be created now (never duplicates a registered tool, never overwrites an earlier created one), else `None`.
pub fn collision(name: &str, registry_names: &[String], data_dir: &Path) -> Option<String> {
    if registry_names.iter().any(|r| r.eq_ignore_ascii_case(name)) {
        return Some(format!("a tool named `{name}` is already registered"));
    }
    if data_dir.join("created_tools").join(name).exists() {
        return Some(format!("a created tool folder for `{name}` already exists"));
    }
    None
}

/// The one-file body: spec + implementation, canonical JSON. This is what is hashed and what a peer's reviewed fetch is answered from —
/// it carries NOTHING about the request that caused it (that is `origin.json`, local only).
pub fn body_json(spec: &ToolSpec) -> String {
    serde_json::to_string_pretty(&json!({
        "format": "ozone-created-tool/1",
        "name": spec.name,
        "description": spec.description,
        "capabilities": spec.capabilities,
        "language": "javascript",
        "input_contract": spec.input_contract,
        "output_contract": spec.output_contract,
        "implementation": spec.implementation,
    }))
    .unwrap_or_default()
}

/// The written artifact: (path of `tool.json`, its body hash, path of `implementation.mjs`).
pub struct Artifact {
    pub body_path: String,
    pub body_hash: String,
    pub implementation_path: String,
}

/// Write the artifact into `<data_dir>/created_tools/<name>/` and NOWHERE else. The folder is created with `create_dir` (an existing one is an
/// error, never reused), every file with `create_new` (never overwritten); on any failure the folder this call created is removed again.
pub fn write_artifact(data_dir: &Path, spec: &ToolSpec, origin: &Value) -> Result<Artifact, String> {
    if sanitize_name(&spec.name).as_deref() != Some(spec.name.as_str()) {
        return Err("the tool name is not a validated slug".to_string());
    }
    let root = data_dir.join("created_tools");
    std::fs::create_dir_all(&root).map_err(|e| format!("cannot create {}: {e}", root.display()))?;
    let dir = root.join(&spec.name);
    std::fs::create_dir(&dir).map_err(|e| format!("cannot create {} (already exists?): {e}", dir.display()))?;
    let write_new = |file: &str, content: &str| -> Result<(), String> {
        let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(dir.join(file)).map_err(|e| format!("cannot create {file}: {e}"))?;
        f.write_all(content.as_bytes()).map_err(|e| format!("cannot write {file}: {e}"))
    };
    let body = body_json(spec);
    let result = write_new("tool.json", &body)
        .and_then(|_| write_new("implementation.mjs", &spec.implementation))
        .and_then(|_| write_new("origin.json", &serde_json::to_string_pretty(origin).unwrap_or_default()));
    if let Err(e) = result {
        let _ = std::fs::remove_dir_all(&dir); // only the folder THIS call just created
        return Err(e);
    }
    Ok(Artifact {
        body_path: dir.join("tool.json").to_string_lossy().to_string(),
        body_hash: crate::network::body_hash(body.as_bytes()),
        implementation_path: dir.join("implementation.mjs").to_string_lossy().to_string(),
    })
}

/// Remove an artifact folder this process just created (used when registration fails after the write). Only a path under
/// `<data_dir>/created_tools/` with a validated slug is ever removed.
pub fn remove_artifact(data_dir: &Path, name: &str) {
    if sanitize_name(name).as_deref() == Some(name) {
        let _ = std::fs::remove_dir_all(data_dir.join("created_tools").join(name));
    }
}

/// The registry row for a created tool: transport `stdio` (the host never spawns it), `auto_start = false`, endpoint = the tool's name.
pub fn registry_row(spec: &ToolSpec) -> crate::mcp::McpTool {
    crate::mcp::McpTool {
        name: spec.name.clone(),
        transport: crate::mcp::McpTransport::Stdio,
        endpoint: spec.name.clone(),
        capabilities: spec.capabilities.clone(),
        server_version: Some("created-0.1".to_string()),
        registered_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        auto_start: false,
    }
}

/// The prompt of the `capability_tool_draft` call site (E30): sees the gap, the request, EVERY registered tool (so it never duplicates one)
/// and the methodology just drafted for the gap; asks for one JSON object.
pub fn draft_prompt(now_line: &str, capability: &str, request: &str, registered: &[String], methodology: Option<&str>) -> String {
    let registered = if registered.is_empty() { "(none)".to_string() } else { registered.join("\n") };
    let methodology = methodology.unwrap_or("(none could be drafted)");
    format!(
        "{now_line}\n\nA plan for the request below needs the capability \"{capability}\", and NO registered tool provides it.\n\n\
         REQUEST (whole):\n{request}\n\nREGISTERED TOOLS (never duplicate one of these):\n{registered}\n\n\
         METHODOLOGY JUST DRAFTED FOR THIS CAPABILITY:\n{methodology}\n\n\
         Specify ONE small, general, reusable tool that provides this capability. It must be about the CAPABILITY, not about this particular \
         request (it may be shared with others): put nothing of the request's own content — names, data, secrets — in the tool. The \
         implementation is a Node.js ES module using only Node built-ins that reads the standardized call envelope \
         {{\"tool\", \"agent\", \"input\", \"context\"}} and returns {{\"success\": true|false, \"output\": ...}}. Be honest: only specify what the \
         implementation really does; if no real tool can provide it, answer {{\"skip\": true}}.\n\n\
         Return ONLY one JSON object: {{\"description\": \"what the tool does\", \"capabilities\": [\"short capability phrases\"], \
         \"language\": \"javascript\", \"input_contract\": {{\"field\": \"type and meaning\"}}, \"output_contract\": {{\"field\": \"type and meaning\"}}, \
         \"implementation\": \"the complete module source\"}}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good() -> Value {
        json!({
            "description": "Reads a graph and reports call edges.",
            "capabilities": ["call graph", " ", "static analysis"],
            "language": "JavaScript",
            "input_contract": {"path": "string — file to analyse"},
            "output_contract": {"edges": "array of [caller, callee]"},
            "implementation": "// handler\nexport default async function handle(envelope) { return { success: true, output: {} }; }\n"
        })
    }

    #[test]
    fn names_are_validated_slugs_and_never_path_like() {
        assert_eq!(sanitize_name("Code CallGraph").as_deref(), Some("code-callgraph"));
        assert_eq!(sanitize_name("rollback-mcp").as_deref(), Some("rollback-mcp"));
        assert_eq!(sanitize_name("../../etc/passwd").as_deref(), Some("etc-passwd"));
        let slug = sanitize_name("../xy").unwrap();
        assert!(!slug.contains('/') && !slug.contains('.'), "no separator or dot survives");
        assert_eq!(sanitize_name("  "), None);
        assert_eq!(sanitize_name("a"), None);
        assert_eq!(sanitize_name("-x-"), None, "separators are trimmed, one character is too short");
    }

    #[test]
    fn spec_parsing_rejects_shells_and_forces_the_gap_name() {
        let spec = parse_spec(&good(), "Code CallGraph").unwrap();
        assert_eq!(spec.name, "code-callgraph", "the name is the gap's slug, never the model's");
        assert!(spec.capabilities.iter().any(|c| c == "Code CallGraph"), "the gap's own name is a capability so a free-text binding finds it");
        assert_eq!(spec.capabilities.iter().filter(|c| c.trim().is_empty()).count(), 0);
        for (key, value) in [
            ("description", json!("...")),
            ("language", json!("python")),
            ("input_contract", json!("a string")),
            ("output_contract", json!({})),
            ("implementation", json!("// only a comment\n")),
            ("implementation", json!("…")),
        ] {
            let mut bad = good();
            bad[key] = value;
            assert!(parse_spec(&bad, "x-tool").is_err(), "{key} must be rejected");
        }
        assert!(parse_spec(&json!({"skip": true}), "x-tool").is_err());
        assert!(parse_spec(&good(), "   ").is_err(), "an unusable gap name creates nothing");
    }

    #[test]
    fn a_registered_or_existing_name_is_a_collision_and_nothing_is_overwritten() {
        let dir = std::env::temp_dir().join(format!("ozone-tool-creation-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let spec = parse_spec(&good(), "call-graph").unwrap();
        assert!(collision("call-graph", &["Call-Graph".to_string()], &dir).is_some(), "an existing registry name (any case)");
        assert!(collision("call-graph", &[], &dir).is_none());
        let art = write_artifact(&dir, &spec, &json!({"gap": "call-graph"})).unwrap();
        assert!(std::path::Path::new(&art.body_path).exists() && std::path::Path::new(&art.implementation_path).exists());
        assert_eq!(art.body_hash, crate::network::body_hash(std::fs::read(&art.body_path).unwrap().as_slice()));
        assert!(collision("call-graph", &[], &dir).is_some(), "the folder now exists");
        assert!(write_artifact(&dir, &spec, &json!({})).is_err(), "a second write never reuses or overwrites the folder");
        let body: Value = serde_json::from_str(&std::fs::read_to_string(&art.body_path).unwrap()).unwrap();
        assert!(body.get("origin").is_none() && body.get("request").is_none(), "the published body carries nothing about the request");
        let origin = std::fs::read_to_string(dir.join("created_tools/call-graph/origin.json")).unwrap();
        assert!(origin.contains("call-graph"));
        remove_artifact(&dir, "call-graph");
        remove_artifact(&dir, "../escape"); // not a slug: nothing is removed
        assert!(!dir.join("created_tools/call-graph").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_registry_row_is_stdio_never_auto_started_and_resolves_by_name() {
        let spec = parse_spec(&good(), "call-graph").unwrap();
        let row = registry_row(&spec);
        assert_eq!(row.transport, crate::mcp::McpTransport::Stdio);
        assert!(!row.auto_start);
        assert_eq!(crate::mcp_supervisor::provider_for_endpoint(&row.endpoint), "call-graph", "a `provider` binding of the gap's name resolves to it");
    }
}
