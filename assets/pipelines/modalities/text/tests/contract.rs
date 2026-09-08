//! Binary contract test — spawns this pipeline exactly as the host executor
//! does: `--input <full PipelineInput JSON> --execution-id <id>`, expects one
//! JSON object on stdout. Covers the envelope unwrap (data/context) and the
//! bare-payload legacy path.

use std::io::{Read, Write};
use std::process::Command;

fn bin_path() -> Option<std::path::PathBuf> {
    // Integration tests of the same package get CARGO_BIN_EXE_<name>;
    // fall back to the shared workspace target dir when run oddly.
    std::env::var_os("CARGO_BIN_EXE_text")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target/debug/text");
            p.exists().then_some(p)
        })
}

#[test]
fn executor_envelope_contract() {
    let Some(bin) = bin_path() else {
        panic!("pipeline binary not built — run `cargo build -p ozone-pipeline-text`");
    };

    // Full PipelineInput envelope, exactly as the host executor passes it.
    let envelope = serde_json::json!({
        "data": {
            "action": {
                "type": "Analyze",
                "text": "Ada Lovelace wrote the first algorithm. Charles Babbage designed the Analytical Engine.",
                "max_chunk_tokens": 2000,
                "depth": "Standard",
                "extract_entities": false,
                "extract_topics": false
            }
        },
        "context": { "user_id": 1, "device_id": 1 }
    });

    let out = Command::new(&bin)
        .arg("--input")
        .arg(envelope.to_string())
        .arg("--execution-id")
        .arg("test-exec-1")
        .output()
        .expect("spawn text pipeline");

    assert!(
        out.status.success(),
        "pipeline exited non-zero: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be one JSON object");
    assert_eq!(parsed["success"], serde_json::json!(true));
    assert!(
        parsed.get("processed_chunks").is_some(),
        "Analyze must emit processed_chunks"
    );
}

#[test]
fn bare_payload_legacy_contract() {
    let Some(bin) = bin_path() else {
        panic!("pipeline binary not built");
    };

    // No envelope — bare tagged action (legacy/standalone callers).
    let bare = serde_json::json!({
        "action": { "type": "GetGraph", "graph_id": 12345 }
    });

    let out = Command::new(&bin)
        .arg("--input")
        .arg(bare.to_string())
        .arg("--execution-id")
        .arg("test-exec-2")
        .output()
        .expect("spawn text pipeline");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be one JSON object");
    // Unknown graph → success:false, but the CONTRACT (parse + execute + JSON
    // out) held; error is reported structurally, not by crashing.
    assert!(parsed.get("success").is_some());
}


#[test]
fn serve_mode_registers_and_executes() {
    let Some(bin) = bin_path() else {
        panic!("pipeline binary not built — run `cargo build -p ozone-pipeline-text`");
    };

    // Mock host: captures the registration, answers 200.
    let host = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let host_addr = host.local_addr().unwrap();
    let host_handle = std::thread::spawn(move || {
        let (mut s, _) = host.accept().unwrap();
        let mut buf = vec![0u8; 8192];
        let n = s.read(&mut buf).unwrap();
        let req = String::from_utf8_lossy(&buf[..n]).to_string();
        // Sanity: it's the register call for pipeline 100.
        assert!(req.contains("/pipelines/register"));
        assert!(req.contains("\"pipeline_id\":100"));
        let body = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 13\r\nConnection: close\r\n\r\n{{\"success\":true}}"
        );
        let _ = s.write_all(body.as_bytes());
        let _ = s.flush();
    });

    // Serve mode: port 0 (prints actual port to stderr), register with mock host.
    let mut child = Command::new(&bin)
        .arg("--serve")
        .arg("--register")
        .arg(format!("http://{host_addr}/pipelines/register"))
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn text pipeline in serve mode");
    // ANY exit path (incl. panics) kills the service — std has no
    // kill_on_drop, so a Drop guard owns the child for this test's scope.
    struct KillOnDrop(std::process::Child);
    impl Drop for KillOnDrop {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut child = KillOnDrop(child);

    // Parse the actual port from "OZONE_PIPELINE_SERVING 100 text <port>".
    // Reader thread accumulates stderr; main loop polls (never blocks on a
    // read, so the deadline is always honored).
    let mut stderr = child.0.stderr.take().unwrap();
    let announce = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let announce_reader = announce.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 512];
        use std::io::Read;
        while let Ok(n) = stderr.read(&mut buf) {
            if n == 0 {
                break;
            }
            announce_reader
                .lock()
                .unwrap()
                .push_str(&String::from_utf8_lossy(&buf[..n]));
        }
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let port = loop {
        if std::time::Instant::now() > deadline {
            panic!("pipeline never announced its serve port; got: {}", announce.lock().unwrap());
        }
        // Complete lines only (a partial chunk may split the port digits).
        let found = announce.lock().unwrap().lines().find_map(|l| {
            if l.starts_with("OZONE_PIPELINE_SERVING") {
                l.split_whitespace().last().and_then(|t| t.parse::<u16>().ok())
            } else {
                None
            }
        });
        if let Some(p) = found {
            break p;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };

    host_handle.join().expect("mock host thread");

    // Dispatch POST /execute through the wire — envelope + action payload.
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    let body = serde_json::json!({
        "data": { "action": { "type": "GetGraph", "graph_id": 7 } },
        "context": {}
    })
    .to_string();
    let request = format!(
        "POST /execute HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(), body
    );
    stream.write_all(request.as_bytes()).unwrap();
    stream.flush().unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();

    // Response body must be pipeline JSON with the success key.
    let json_start = response.find("\r\n\r\n").map(|i| i + 4).expect("HTTP response body");
    let parsed: serde_json::Value =
        serde_json::from_str(response[json_start..].trim()).expect("JSON output");
    assert!(parsed.get("success").is_some());

    // Drop of KillOnDrop kills + reaps — no orphan outlives the test.
    drop(child);
}
