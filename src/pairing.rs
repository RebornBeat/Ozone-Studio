//! QR device pairing — the phone as authenticator.
//!
//! Ozone-Studio is multi-device: the same host serves the desktop UI, a web
//! UI, and (eventually) the mobile app. New devices join securely without
//! passwords or typed secrets:
//!
//! 1. The device asks the host to start a pairing session and shows the
//!    returned QR code (plus a human-readable code).
//! 2. The user's phone — already trusted, already in hand — scans it. The QR
//!    encodes a short-lived approve URL on the host itself.
//! 3. The phone approves with one tap. The host creates a REAL session
//!    through the existing `AuthSystem` (same session store, same expiry
//!    policy as Ed25519 logins) and attaches a registered device.
//! 4. The pairing device, polling `/pairing/status`, receives the session
//!    token and is in — multi-device, no key ever typed or displayed.
//!
//! Possession of the QR is the factor: the code is 8 unambiguous chars, the
//! session expires in minutes, and approval requires reaching the host.

use crate::auth::AuthSystem;
use crate::types::auth::{DeviceType, Session};
use rand::RngCore;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Lifetime of a pending pairing session (seconds).
const PAIRING_TTL_SECS: u64 = 300;

/// Alphabet without visually ambiguous characters (no 0/O/1/I/L).
const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
const CODE_LEN: usize = 8;

/// A pending or approved pairing session.
#[derive(Debug, Clone)]
pub struct PairingSession {
    pub pairing_id: String,
    pub code: String,
    pub device_hint: String,
    pub created_at: u64,
    pub expires_at: u64,
    /// Set when the phone approves — hex session token for the polling device.
    pub approved_token_hex: Option<String>,
    pub approved_device_id: Option<u64>,
}

/// Registry of pairing sessions. Tokens themselves live in the `AuthSystem`
/// session store, so every existing validation path works unchanged.
pub struct PairingHub {
    sessions: Arc<RwLock<HashMap<String, PairingSession>>>,
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn random_code() -> String {
    let mut rng = rand::thread_rng();
    (0..CODE_LEN)
        .map(|_| {
            let i = rng.next_u32() as usize % CODE_ALPHABET.len();
            CODE_ALPHABET[i] as char
        })
        .collect()
}

fn random_hex(n: usize) -> String {
    let mut buf = vec![0u8; n];
    rand::thread_rng().fill_bytes(&mut buf);
    buf.iter().map(|b| format!("{:02x}", b)).collect()
}

impl PairingHub {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start a pairing session; returns (pairing_id, code, expires_at).
    pub async fn start(&self, device_hint: String) -> (String, String, u64) {
        // Opportunistic prune of stale sessions.
        self.prune().await;

        let pairing_id = random_hex(16);
        let code = random_code();
        let created = now_secs();
        let expires_at = created + PAIRING_TTL_SECS;

        self.sessions.write().await.insert(
            pairing_id.clone(),
            PairingSession {
                pairing_id: pairing_id.clone(),
                code,
                device_hint,
                created_at: created,
                expires_at,
                approved_token_hex: None,
                approved_device_id: None,
            },
        );

        let code_out = {
            let sessions = self.sessions.read().await;
            sessions
                .get(&pairing_id)
                .map(|s| s.code.clone())
                .unwrap_or_default()
        };
        (pairing_id, code_out, expires_at)
    }

    /// Poll pairing status. Returns (status, token_hex_if_approved, device_id, expires_at).
    pub async fn status(&self, pairing_id: &str) -> (String, Option<String>, Option<u64>, Option<u64>) {
        let sessions = self.sessions.read().await;
        match sessions.get(pairing_id) {
            None => ("unknown".into(), None, None, None),
            Some(s) => {
                if s.approved_token_hex.is_some() {
                    (
                        "approved".into(),
                        s.approved_token_hex.clone(),
                        s.approved_device_id,
                        Some(s.expires_at),
                    )
                } else if now_secs() > s.expires_at {
                    ("expired".into(), None, None, Some(s.expires_at))
                } else {
                    ("pending".into(), None, None, Some(s.expires_at))
                }
            }
        }
    }

    /// Phone approval: consume `code`, create a real session for the device
    /// through the host's `AuthSystem`, and release the token to the polling
    /// device. Returns the device id that was registered.
    pub async fn approve(
        &self,
        code: &str,
        device_name: String,
        device_type: DeviceType,
        auth: &AuthSystem,
    ) -> Result<u64, String> {
        let pairing_id = {
            let sessions = self.sessions.read().await;
            let now = now_secs();
            let found = sessions
                .values()
                .find(|s| s.code == code.to_uppercase() && now <= s.expires_at)
                .map(|s| s.pairing_id.clone());
            match found {
                Some(id) => id,
                None => return Err("Invalid or expired pairing code".into()),
            }
        };

        let session: Session = auth
            .create_pairing_session(device_name, device_type)
            .await
            .map_err(|e| e.to_string())?;
        let device_id = session.device_id;
        let token_hex = hex::encode(&session.session_token);

        let mut sessions = self.sessions.write().await;
        if let Some(s) = sessions.get_mut(&pairing_id) {
            s.approved_token_hex = Some(token_hex);
            s.approved_device_id = Some(device_id);
        }
        Ok(device_id)
    }

    async fn prune(&self) {
        let now = now_secs();
        self.sessions
            .write()
            .await
            .retain(|_, s| now <= s.expires_at + 60);
    }
}

impl Default for PairingHub {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve the address phones should reach for the approve page: explicit
/// `OZONE_PAIRING_HOST` override, else the LAN IP seen by the OS (UDP
/// connect trick — no packets are sent), else loopback.
pub fn pairing_public_host(fallback: &str) -> String {
    if let Ok(host) = std::env::var("OZONE_PAIRING_HOST") {
        if !host.is_empty() {
            return host;
        }
    }
    if let Ok(addr) = detect_lan_ip() {
        return addr;
    }
    fallback.to_string()
}

fn detect_lan_ip() -> Result<String, std::io::Error> {
    let sock = std::net::UdpSocket::bind("0.0.0.0:0")?;
    // Connect sets the default route destination without sending packets.
    sock.connect("8.8.8.8:80")?;
    let ip = sock.local_addr()?.ip().to_string();
    Ok(ip)
}

/// Phone-facing approve page — served by the host at GET /pair/{code}.
/// Scanned by the phone camera; one tap approves the waiting device.
pub fn approve_page_html(code: &str, host_label: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Ozone-Studio Pairing</title>
<style>
  body {{ font-family: -apple-system, sans-serif; background: #0b0e14; color: #e8e8e8;
         display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }}
  .card {{ background: #151a24; border: 1px solid #2a3242; border-radius: 14px;
           padding: 28px; max-width: 340px; width: 90%; text-align: center; }}
  .code {{ font-family: monospace; font-size: 26px; letter-spacing: 3px; color: #6ec3ff; margin: 12px 0; }}
  .hint {{ opacity: 0.65; font-size: 13px; margin-bottom: 20px; }}
  button {{ background: #2ecc71; border: 0; color: #06220f; font-size: 17px; font-weight: 600;
            padding: 13px 34px; border-radius: 10px; width: 100%; }}
  button.deny {{ background: transparent; color: #e74c3c; margin-top: 10px; font-weight: 400; }}
  .done {{ color: #2ecc71; font-size: 20px; font-weight: 600; }}
  .err {{ color: #e74c3c; }}
</style>
</head>
<body>
  <div class="card">
    <div id="stage-ask">
      <h2>Pair a device?</h2>
      <div class="hint">Host: {host_label}<br>A device is waiting with this code:</div>
      <div class="code">{code}</div>
      <button onclick="approve()">Approve pairing</button>
      <button class="deny" onclick="deny()">Deny</button>
    </div>
    <div id="stage-done" style="display:none">
      <div class="done">&#10003; Device paired</div>
      <div class="hint" style="margin-top:10px">It now holds a session on this host. You can close this page.</div>
    </div>
    <div id="stage-err" style="display:none">
      <div class="err" id="err-msg"></div>
    </div>
  </div>
<script>
async function post(url, body) {{
  const r = await fetch(url, {{ method: "POST",
    headers: {{ "Content-Type": "application/json" }}, body: JSON.stringify(body ?? {{}}) }});
  if (!r.ok) throw new Error("HTTP " + r.status);
  return r.json();
}}
async function approve() {{
  try {{
    const name = "Phone (" + new Date().toLocaleTimeString() + ")";
    await post("/pairing/approve", {{ code: "{code}", device_name: name }});
    document.getElementById("stage-ask").style.display = "none";
    document.getElementById("stage-done").style.display = "block";
  }} catch (e) {{
    document.getElementById("stage-ask").style.display = "none";
    document.getElementById("stage-err").style.display = "block";
    document.getElementById("err-msg").textContent = "Pairing failed: " + e.message;
  }}
}}
async function deny() {{
  document.getElementById("stage-ask").style.display = "none";
  document.getElementById("stage-err").style.display = "block";
  document.getElementById("err-msg").textContent = "Pairing denied. The waiting device receives nothing.";
}}
</script>
</body>
</html>"#,
        host_label = host_label,
        code = code
    )
}
