//! Thin CLI for the UiDocument pipeline (one-shot contract used by the other
//! modality pipelines): the host passes the {"data": {...}, "context": {...}}
//! envelope via `--input '<json>'` (or the bare payload; or stdin when no
//! --input is given) and reads exactly one JSON object from stdout.
//!
//!   uidocument --input '{"data":{"flavor":"html","html":"<p>hi</p>"},"context":{}}'
//!   echo '{"flavor":"css","css":["a{color:red}"]}' | uidocument
//!
//! Exit code 0 on success, 1 on any failure (the JSON still says why).

use std::io::Read;

use serde_json::{json, Value};

fn unwrap_envelope(v: Value) -> Value {
    if v.get("context").is_some() {
        if let Some(inner) = v.get("data") {
            return inner.clone();
        }
    }
    v
}

fn read_input() -> Result<String, String> {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--input" && i + 1 < args.len() {
            return Ok(args[i + 1].clone());
        }
        if args[i] == "--input-file" && i + 1 < args.len() {
            return std::fs::read_to_string(&args[i + 1]).map_err(|e| format!("cannot read {}: {}", args[i + 1], e));
        }
        i += 1;
    }
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).map_err(|e| format!("cannot read stdin: {}", e))?;
    if s.trim().is_empty() {
        return Err("no input: pass --input '<json>' or pipe JSON on stdin".to_string());
    }
    Ok(s)
}

fn main() {
    let raw = match read_input() {
        Ok(r) => r,
        Err(e) => {
            println!("{}", json!({"success": false, "error": e, "pipeline_id": uidocument::PIPELINE_ID}));
            std::process::exit(1);
        }
    };
    let parsed: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", json!({"success": false, "error": format!("Parse error: {}", e), "pipeline_id": uidocument::PIPELINE_ID}));
            std::process::exit(1);
        }
    };
    let input = unwrap_envelope(parsed);
    // Input is untrusted: a panic in analysis must still end as an honest JSON answer.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| uidocument::analyze(&input)));
    let out = match result {
        Ok(v) => v,
        Err(_) => json!({"success": false, "error": "internal error while analyzing this input", "pipeline_id": uidocument::PIPELINE_ID}),
    };
    let ok = out.get("success").and_then(|s| s.as_bool()).unwrap_or(false);
    println!("{}", out);
    if !ok {
        std::process::exit(1);
    }
}
