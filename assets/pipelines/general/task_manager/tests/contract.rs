//! Binary contract test — the host executor spawns this binary with
//! `--input <full PipelineInput JSON> --execution-id <id>`; stdout must be
//! one JSON object. Verifies the envelope unwrap and the execute path.

use std::process::Command;

fn bin_path() -> Option<std::path::PathBuf> {
    std::env::var_os("CARGO_BIN_EXE_task_manager")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target/debug/task_manager");
            p.exists().then_some(p)
        })
}

#[test]
fn executor_envelope_contract() {
    let Some(bin) = bin_path() else {
        panic!("pipeline binary not built — run `cargo build -p ozone-pipeline-task-manager`");
    };

    // Full PipelineInput envelope with a frontend-style flat action payload.
    // NOTE: run in a temp cwd — the task store writes ./data relative to cwd.
    let tmp = std::env::temp_dir().join(format!("oz-tm-test-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();

    let envelope = serde_json::json!({
        "data": {
            "action": "GetDetails",
            "task_id": 987654321u64
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
        .expect("spawn task_manager pipeline");

    assert!(
        out.status.success(),
        "pipeline exited non-zero: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be one JSON object");
    // Unknown task → success:false reported STRUCTURALLY (task_id null +
    // error string), which proves the full contract: envelope parse →
    // dispatch → execute → JSON stdout, with failure as data not a crash.
    assert_eq!(parsed["success"], serde_json::json!(false));
    assert!(
        parsed["error"]
            .as_str()
            .map(|e| e.contains("not found"))
            .unwrap_or(false),
        "expected a not-found error, got: {}",
        parsed["error"]
    );

    let _ = std::fs::remove_dir_all(&tmp);
}
