//! Ozone pipeline serve mode — dependency-free (std only).
//!
//! Include from a pipeline's main.rs:
//!   #[path = "../../shared/ozone_serve.rs"]
//!   mod ozone_serve;
//!
//! Then in main():
//!   if let Some(opts) = ozone_serve::serve_mode() {
//!       ozone_serve::serve(opts, pipeline_id, name, handler);
//!       return;
//!   }
//!
//! The handler receives the UNWRAPPED action payload (the `data` envelope is
//! removed, same as the one-shot path) and returns the output JSON object —
//! the exact contract the host executor and CLI path speak. While serving,
//! the pipeline registers itself with the host (`--register <url>`, optional)
//! and re-announces on a heartbeat so the host's dispatch table stays live.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

/// Parsed `--serve` options.
pub struct ServeOptions {
    pub port: u16,
    /// Host registration endpoint, e.g. http://127.0.0.1:8080/pipelines/register
    pub register_url: Option<String>,
}

/// Detect `--serve [--port N] [--register <url>]` in the process args.
/// Returns None when this invocation is a normal one-shot run.
pub fn serve_mode() -> Option<ServeOptions> {
    let args: Vec<String> = std::env::args().collect();
    if !args.iter().any(|a| a == "--serve") {
        return None;
    }
    let mut port = 0u16; // 0 = OS-assigned; actual printed to stderr
    let mut register_url = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" if i + 1 < args.len() => {
                port = args[i + 1].parse().unwrap_or(0);
                i += 2;
            }
            "--register" if i + 1 < args.len() => {
                register_url = Some(args[i + 1].clone());
                i += 2;
            }
            _ => i += 1,
        }
    }
    Some(ServeOptions { port, register_url })
}

/// Serve `POST /execute` (body: PipelineInput JSON) until killed.
/// `handler` receives the unwrapped `data` payload and returns the output.
pub fn serve(
    opts: ServeOptions,
    pipeline_id: u64,
    name: String,
    handler: Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync>,
) -> ! {
    let listener =
        TcpListener::bind(("127.0.0.1", opts.port)).expect("serve: failed to bind port");
    let actual_port = listener.local_addr().expect("serve: local addr").port();

    // Announce the actual port (tests and supervisors parse this).
    eprintln!("OZONE_PIPELINE_SERVING {pipeline_id} {name} {actual_port}");

    if let Some(url) = &opts.register_url {
        register_with_host(url, pipeline_id, &name, actual_port);
        // Heartbeat: re-announce so the host's dispatch table stays current.
        let url = url.clone();
        let name = name.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(30));
            register_with_host(&url, pipeline_id, &name, actual_port);
        });
    }

    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let handler = handler.clone();
        let name = name.clone();
        std::thread::spawn(move || {
            let request = match read_request(&mut stream) {
                Some(r) => r,
                None => return,
            };
            let response = if request.path == "/execute" && request.method == "POST" {
                // Same envelope contract as the one-shot path: unwrap `data`.
                let envelope: serde_json::Value =
                    serde_json::from_slice(&request.body).unwrap_or(serde_json::json!({}));
                let action_payload = envelope.get("data").cloned().unwrap_or(envelope);
                handler(action_payload)
            } else if request.path == "/health" {
                serde_json::json!({ "ok": true, "pipeline_id": pipeline_id, "name": name })
            } else {
                serde_json::json!({ "success": false, "error": "not found" })
            };
            write_response(&mut stream, &response);
        });
    }

    unreachable!("listener.incoming() never returns None")
}

struct HttpRequest {
    #[allow(dead_code)]
    method: String,
    path: String,
    body: Vec<u8>,
}

fn read_request(stream: &mut TcpStream) -> Option<HttpRequest> {
    let mut buf = vec![0u8; 65536];
    let mut total = 0usize;
    // Read until end of headers.
    let header_end = loop {
        if total >= buf.len() {
            return None;
        }
        let n = stream.read(&mut buf[total..]).ok()?;
        if n == 0 {
            return None;
        }
        total += n;
        if let Some(pos) = find_subsequence(&buf[..total], b"\r\n\r\n") {
            break pos + 4;
        }
    };
    let headers = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = headers.split("\r\n");
    let request_line = lines.next()?.to_string();
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();

    // Content-Length body.
    let content_length = headers
        .to_lowercase()
        .lines()
        .find_map(|l| l.strip_prefix("content-length:"))
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = buf[header_end..total].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&buf[..n]);
    }
    body.truncate(content_length);

    Some(HttpRequest { method, path, body })
}

fn write_response(stream: &mut TcpStream, json: &serde_json::Value) {
    let body = serde_json::to_string(json).unwrap_or_default();
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

/// Minimal raw-TCP JSON POST (no deps) for host registration + heartbeat.
fn register_with_host(register_url: &str, pipeline_id: u64, name: &str, port: u16) {
    // Split host:port from path BEFORE connect — ToSocketAddrs rejects a
    // trailing path component.
    let bare = register_url.trim_start_matches("http://").trim_start_matches("https://");
    let (hostport, path) = match bare.find('/') {
        Some(i) => (&bare[..i], &bare[i..]),
        None => (bare, "/"),
    };
    let Ok(mut stream) = TcpStream::connect(hostport) else {
        eprintln!("ozone_serve: host not reachable at {register_url} (will retry)");
        return;
    };
    let body = serde_json::json!({
        "pipeline_id": pipeline_id,
        "name": name,
        "execute_url": format!("http://127.0.0.1:{port}/execute"),
    })
    .to_string();
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {hostport}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(request.as_bytes());
    let _ = stream.flush();
    // Drain the response (best-effort; failures retried by heartbeat).
    let mut buf = vec![0u8; 4096];
    let _ = stream.read(&mut buf);
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}
