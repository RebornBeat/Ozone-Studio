//! Connect-model integration tests: a pipeline registers itself over HTTP,
//! the executor dispatches to it remote-FIRST (never spawning), and
//! deregistration falls back cleanly.

use ozone_studio::pipeline::{PipelineExecutor, RemotePipelines};
use ozone_studio::types::pipeline::{ExecutionID, PipelineInput};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;

/// One-shot HTTP responder: consumes exactly one POST and replies with a
/// canned JSON output object (the same contract a real pipeline serves).
fn spawn_canned_pipeline_server(response_json: String) -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let addr = listener.local_addr().expect("local addr");

    let handle = std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = vec![0u8; 65536];
            let _ = stream.read(&mut buf);
            let body = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_json.len(),
                response_json
            );
            let _ = stream.write_all(body.as_bytes());
            let _ = stream.flush();
        }
    });

    (format!("http://{addr}/execute"), handle)
}

fn test_executor() -> PipelineExecutor {
    let tmp = std::env::temp_dir().join(format!("oz-remote-test-{}", std::process::id()));
    std::fs::create_dir_all(tmp.join("builtin")).unwrap();
    std::fs::create_dir_all(tmp.join("custom")).unwrap();
    let config = ozone_studio::config::PipelineConfig {
        builtin_path: tmp.join("builtin").to_string_lossy().to_string(),
        custom_path: tmp.join("custom").to_string_lossy().to_string(),
        max_concurrent_pipelines: 4,
        index_path: tmp.join("no-index.json").to_string_lossy().to_string(),
    };
    PipelineExecutor::new(&config).expect("executor")
}

#[tokio::test]
async fn register_execute_and_count() {
    let executor = test_executor();
    let remotes: std::sync::Arc<RemotePipelines> = executor.remote_pipelines();

    let (url, server) = spawn_canned_pipeline_server(
        r#"{"success": true, "result": "remote-worked"}"#.to_string(),
    );

    // The pipeline "connects": it announces itself.
    let entry = remotes.register(999, "test-remote".to_string(), url).await;
    assert_eq!(entry.pipeline_id, 999);

    // Host dispatch lands on the remote, not on a spawn path.
    let input = PipelineInput {
        data: HashMap::new(),
        context: Default::default(),
    };
    let output = remotes
        .execute(999, &input, ExecutionID::new(), None)
        .await
        .expect("remote dispatch");

    assert!(output.success);
    assert_eq!(
        output.data.get("result"),
        Some(&serde_json::json!("remote-worked"))
    );

    let listed = remotes.list().await;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].call_count, 1);

    server.join().expect("responder thread");
}

#[tokio::test]
async fn deregister_falls_back_to_absent() {
    let remotes = RemotePipelines::new();

    remotes
        .register(1000, "gone-soon".to_string(), "http://127.0.0.1:1/x".to_string())
        .await;
    assert!(remotes.deregister(1000).await);
    assert!(!remotes.deregister(1000).await); // second deregister: nothing there
    assert!(remotes.get(1000).await.is_none());
    assert!(remotes.is_empty().await);

    // Dispatch with no registration is an Err (executor would fall back to spawn).
    let input = PipelineInput {
        data: HashMap::new(),
        context: Default::default(),
    };
    assert!(remotes.execute(1000, &input, ExecutionID::new(), None).await.is_err());
}


#[tokio::test]
async fn search_registry_default_is_selectable() {
    let registry = ozone_studio::zsei::search::SearchRegistry::new();
    assert!(registry.set_default("exact").await);
    assert!(registry.set_default("scan").await);
    assert!(!registry.set_default("nonexistent").await);
}
