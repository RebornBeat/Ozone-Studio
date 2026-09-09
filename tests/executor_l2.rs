//! L2 — full `PipelineExecutor::execute` against a real blueprint on BOTH
//! dispatch paths:
//!   1. custom spawn path (binary copied to the configured custom dir)
//!   2. remote dispatch (registered pipeline wins; spawn prevented)
//!
//! The blueprint fixture uses genuinely-computed placeholder identity
//! (empty key, zero hash) — dispatch reads only `pipeline_id` + `name`.

mod tempdir;
use ozone_studio::pipeline::PipelineExecutor;
use ozone_studio::types::pipeline::{
    BlueprintSpec, ExecutionFlow, PipelineBlueprint, PipelineInput, Schema,
};
use ozone_studio::types::{Blake3Hash, PublicKey, SemVer, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;

// ── fixtures ──────────────────────────────────────────────────────────────

fn fixture_blueprint(pipeline_id: u64, name: &str) -> PipelineBlueprint {
    PipelineBlueprint {
        pipeline_id,
        name: name.to_string(),
        version: SemVer::default(),
        author: Vec::<u8>::new(), // PublicKey = Vec<u8>
        description: "L2 executor dispatch fixture".into(),
        specification: BlueprintSpec {
            input_schema: Schema { fields: vec![], validation_rules: vec![] },
            output_schema: Schema { fields: vec![], validation_rules: vec![] },
            dependencies: vec![],
            sub_pipelines: vec![],
            execution_flow: ExecutionFlow::Sequential(vec![pipeline_id]),
        },
        implementations: vec![],
        content_hash: [0u8; 32], // Blake3Hash = [u8; 32] — fixture, not persisted
        peers: vec![],
        consensus_status: ozone_studio::types::pipeline::ConsensusStatus::Open,
        verified_by: 0,
    }
}

fn text_binary_path() -> Option<std::path::PathBuf> {
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../assets/pipelines/modalities/text/target/debug/text",
    );
    p.canonicalize().ok()
}

fn executor_with_temp_paths(tag: &str) -> (PipelineExecutor, self::tempdir::TempDir) {
    let tmp = std::env::temp_dir().join(format!("oz-l2-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(tmp.join("builtin")).unwrap();
    std::fs::create_dir_all(tmp.join("custom")).unwrap();
    let config = ozone_studio::config::PipelineConfig {
        builtin_path: tmp.join("builtin").to_string_lossy().to_string(),
        custom_path: tmp.join("custom").to_string_lossy().to_string(),
        max_concurrent_pipelines: 4,
        index_path: tmp.join("no-index.json").to_string_lossy().to_string(),
    };
    let exec = PipelineExecutor::new(&config).expect("executor");
    (exec, tempdir::TempDir(tmp))
}

// Tiny RAII so temp dirs clean up even on panic.
struct TempDir(std::path::PathBuf);
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn analyze_input(text: &str) -> PipelineInput {
    let action = serde_json::json!({
        "type": "Analyze",
        "text": text,
        "max_chunk_tokens": 2000,
    });
    let mut data = HashMap::new();
    data.insert(
        "action".to_string(),
        Value::Map(
            action
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), json_to_value(v)))
                .collect(),
        ),
    );
    PipelineInput { data, context: Default::default() }
}

fn json_to_value(v: &serde_json::Value) -> Value {
    use crate_alias::JsonExt;
    v.to_typed_value()
}

mod crate_alias {
    pub trait JsonExt {
        fn to_typed_value(&self) -> ozone_studio::types::Value;
    }
    impl JsonExt for serde_json::Value {
        fn to_typed_value(&self) -> ozone_studio::types::Value {
            use ozone_studio::types::Value as V;
            match self {
                serde_json::Value::Null => V::Null,
                serde_json::Value::Bool(b) => V::Bool(*b),
                serde_json::Value::Number(n) if n.is_i64() => V::Int(n.as_i64().unwrap_or(0)),
                serde_json::Value::Number(n) => V::Float(n.as_f64().unwrap_or(0.0)),
                serde_json::Value::String(s) => V::String(s.clone()),
                serde_json::Value::Array(a) => {
                    V::Array(a.iter().map(|x| x.to_typed_value()).collect())
                }
                serde_json::Value::Object(m) => V::Map(
                    m.iter().map(|(k, v)| (k.clone(), v.to_typed_value())).collect(),
                ),
            }
        }
    }
}
use crate_alias::JsonExt;

// ── path 1: custom spawn ─────────────────────────────────────────────────

#[tokio::test]
async fn l2_custom_spawn_executes_real_binary() {
    let Some(bin) = text_binary_path() else {
        eprintln!("SKIP: text binary not built — run `cargo build --manifest-path assets/pipelines/modalities/text/Cargo.toml`");
        return;
    };

    let (exec, _tmp) = executor_with_temp_paths("spawn");
    let custom_dir = _tmp.0.join("custom");
    std::fs::copy(&bin, custom_dir.join("text")).expect("copy text binary");

    let bp = fixture_blueprint(100, "text");
    let input = analyze_input("Ada Lovelace wrote the first algorithm for the Analytical Engine.");

    let output = exec.execute(&bp, input, None).await.expect("spawn execute");
    assert!(output.success, "spawn failed: {:?}", output.error);
    assert!(
        output.data.contains_key("processed_chunks"),
        "text Analyze output must carry processed_chunks; got keys: {:?}",
        output.data.keys().collect::<Vec<_>>()
    );
}

// ── path 2: remote dispatch wins over spawn ──────────────────────────────

fn canned_server(response: String) -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        if let Ok((mut s, _)) = listener.accept() {
            let mut buf = vec![0u8; 16384];
            let _ = s.read(&mut buf);
            let body = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response.len(),
                response
            );
            let _ = s.write_all(body.as_bytes());
            let _ = s.flush();
        }
    });
    (format!("http://{addr}/execute"), handle)
}

#[tokio::test]
async fn l2_remote_dispatch_wins_over_spawn() {
    let (exec, _tmp) = executor_with_temp_paths("remote");
    let remotes = exec.remote_pipelines();

    let (url, server) =
        canned_server(r#"{"success": true, "source": "remote-dispatch"}"#.to_string());
    remotes.register(100, "text-remote".to_string(), url).await;

    let bp = fixture_blueprint(100, "text");
    let output = exec.execute(&bp, analyze_input("anything"), None).await.unwrap();

    assert!(output.success);
    assert_eq!(
        output.data.get("source"),
        Some(&serde_json::json!("remote-dispatch")),
        "remote must win the dispatch — spawn would have run the real binary"
    );

    server.join().unwrap();
}
