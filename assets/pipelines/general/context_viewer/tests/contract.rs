//! Binary contract test — the host executor spawns this binary with
//! `--input <full PipelineInput JSON> --execution-id <id>`; stdout must be
//! one JSON object. Verifies the envelope unwrap on a read-only action.

use std::process::Command;

fn bin_path() -> Option<std::path::PathBuf> {
    std::env::var_os("CARGO_BIN_EXE_context_viewer")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target/debug/context_viewer");
            p.exists().then_some(p)
        })
}

#[test]
fn executor_envelope_contract() {
    let Some(bin) = bin_path() else {
        panic!("pipeline binary not built — run `cargo build -p ozone-pipeline-context-viewer`");
    };

    // Run in a temp cwd — the context viewer writes ./data relative to cwd.
    let tmp = std::env::temp_dir().join(format!("oz-cv-test-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();

    let envelope = serde_json::json!({
        "data": {
            "action": "GetTaskGraphs",
            "task_id": 42
        },
        "context": { "user_id": 1, "device_id": 1 }
    });

    let out = Command::new(&bin)
        .arg("--input")
        .arg(envelope.to_string())
        .arg("--execution-id")
        .arg("test-exec-1")
        .current_dir(&tmp)
        .output()
        .expect("spawn context_viewer pipeline");

    assert!(
        out.status.success(),
        "pipeline exited non-zero: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be one JSON object");
    // Empty store → success:true with an empty graphs list.
    assert_eq!(parsed["success"], serde_json::json!(true));
    assert_eq!(parsed["graphs"], serde_json::json!([]));

    let _ = std::fs::remove_dir_all(&tmp);
}
