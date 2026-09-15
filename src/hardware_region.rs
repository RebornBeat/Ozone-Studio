//! Hardware/OS-derived region detection for the jurisdiction gate.
//!
//! This is NOT legal content and does not decide what law applies — it only
//! answers "what does real, observable evidence say about where this
//! machine is," using real signals (tzdata's own zone.tab country mapping,
//! POSIX locale naming, and — see below — this host's own public IP via a
//! real geolocation lookup), never a fabricated or hardcoded lookup table.
//!
//! Deliberately conservative: three independent signals (system timezone,
//! system locale, IP geolocation) are read and cross-checked, and a region
//! is only trusted when at least two of the available signals agree with
//! each other (see `agreed_region`'s doc comment for the exact rule and why
//! a plain "all must agree" or "trust the first one" rule was rejected).
//! Signals can genuinely disagree (e.g. a US-locale machine physically
//! hosted with an unrelated system timezone) — confirmed live on the very
//! machine this was built on: timezone AND IP geolocation both resolved to
//! the Dominican Republic while locale resolved to the US. Since this value
//! ends up driving which jurisdiction rules can BLOCK a request, a wrong
//! silent guess is worse than no guess.
//!
//! IP geolocation specifically: this makes one real outbound HTTP call, at
//! most once at config load, to a real public IP-geolocation API
//! (ip-api.com's free, keyless endpoint — see `detect_ip_country`) asking
//! only "what country is MY OWN public IP in" — no request-specific or
//! per-user data is ever sent, only this host's own connection. It is
//! deliberately NOT a locally-bundled IP database (e.g. MaxMind
//! GeoLite2-Country): that would require either shipping a multi-MB binary
//! blob in this repo that goes stale the moment IP allocations shift (a
//! fabricated-lookup risk of exactly the kind this module's design
//! otherwise avoids), or a MaxMind account/license key this environment
//! doesn't have — a live lookup against a real, currently-accurate service
//! is more honest than a bundled table pretending to be current. The call
//! is short-timeout (3s) and strictly non-fatal: any failure (offline, the
//! service down, a bad response) yields None for this one signal and never
//! blocks or delays boot beyond that timeout — the other two signals (or
//! an explicit config.toml value) still work with zero dependency on this
//! call succeeding.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

/// Real signals read from the OS/network, kept separate so a caller can log
/// exactly what was seen even when they disagree.
#[derive(Debug, Clone, Default)]
pub struct HardwareRegionSignals {
    pub timezone_name: Option<String>,
    pub timezone_country: Option<String>,
    pub locale_raw: Option<String>,
    pub locale_country: Option<String>,
    pub ip_country: Option<String>,
}

impl HardwareRegionSignals {
    /// Some(region) only when at least TWO of the (up to three) available
    /// signals agree with each other; otherwise None.
    ///
    /// Why 2-of-available rather than "all must agree" or "trust any one
    /// signal": requiring unanimous agreement across three independently
    /// sourced signals is fragile — any one signal being merely unavailable
    /// (e.g. the IP call times out) would silently degrade coverage even
    /// when the two real answers that DID come back agree with each other.
    /// Trusting a single signal alone is exactly what this module's design
    /// has rejected from the start (a wrong silent guess is worse than no
    /// guess for a value that can BLOCK real requests). Two independent,
    /// agreeing signals — out of however many actually resolved — is the
    /// same conservative bar the original two-signal design used, extended
    /// to a third signal rather than replaced by it. A single available
    /// signal, or three signals that all disagree, both correctly yield
    /// None.
    pub fn agreed_region(&self) -> Option<String> {
        let available: Vec<&str> = [
            self.timezone_country.as_deref(),
            self.locale_country.as_deref(),
            self.ip_country.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect();

        for (i, a) in available.iter().enumerate() {
            for b in &available[i + 1..] {
                if a.eq_ignore_ascii_case(b) {
                    return Some(a.to_uppercase());
                }
            }
        }
        None
    }
}

/// System's own timezone name, e.g. "America/New_York". Tries the portable
/// `/etc/localtime` symlink target first (works across distros), falls back
/// to `/etc/timezone` (Debian/Ubuntu convention), then the `TZ` env var.
fn read_system_timezone_name() -> Option<String> {
    if let Ok(target) = std::fs::read_link("/etc/localtime") {
        let s = target.to_string_lossy();
        if let Some(idx) = s.find("zoneinfo/") {
            return Some(s[idx + "zoneinfo/".len()..].to_string());
        }
    }
    if let Ok(content) = std::fs::read_to_string("/etc/timezone") {
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    std::env::var("TZ").ok().filter(|s| !s.is_empty())
}

/// Parses tzdata's own zone1970.tab/zone.tab (real system file, ships with
/// every Linux tzdata install — not a table maintained here) into a
/// {timezone_name -> ISO 3166 country code} map, memoized per-process since
/// the file never changes during a run.
fn timezone_country_table() -> &'static HashMap<String, String> {
    static TABLE: OnceLock<HashMap<String, String>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut map = HashMap::new();
        let path = if std::path::Path::new("/usr/share/zoneinfo/zone1970.tab").exists() {
            "/usr/share/zoneinfo/zone1970.tab"
        } else {
            "/usr/share/zoneinfo/zone.tab"
        };
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                if line.starts_with('#') || line.trim().is_empty() {
                    continue;
                }
                let fields: Vec<&str> = line.split('\t').collect();
                // zone.tab: country_codes, coordinates, zone_name[, comments]
                // zone1970.tab has the same first three fields.
                if fields.len() >= 3 {
                    let codes = fields[0];
                    let zone_name = fields[2];
                    // A row can list multiple comma-separated country codes
                    // for one zone; take the first as the primary mapping.
                    if let Some(primary) = codes.split(',').next() {
                        map.insert(zone_name.to_string(), primary.to_string());
                    }
                }
            }
        }
        map
    })
}

/// System locale's country code, from the standard POSIX
/// `language[_COUNTRY][.encoding]` naming (e.g. "en_US.UTF-8" -> "US").
/// Tries LC_ALL, then LANG (their real precedence order).
fn read_locale_country() -> (Option<String>, Option<String>) {
    let raw = std::env::var("LC_ALL")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("LANG").ok().filter(|s| !s.is_empty()));
    let country = raw.as_ref().and_then(|s| {
        let lang_part = s.split('.').next().unwrap_or(s);
        lang_part.split_once('_').map(|(_, country)| country.to_uppercase())
    });
    (raw, country)
}

/// One real outbound call to ip-api.com's free, keyless JSON endpoint,
/// asking only for this host's own connection's country — see the module
/// doc comment for why this is a live lookup rather than a bundled
/// database, and why it's strictly non-fatal. `fields=status,countryCode`
/// requests the minimal real response shape; anything other than a clean
/// `{"status":"success","countryCode":"XX"}` (network error, timeout,
/// non-2xx, malformed body, or an explicit `"status":"fail"`) yields None —
/// never a guess, never a partial/fabricated code.
async fn detect_ip_country() -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .ok()?;
    let resp = client
        .get("http://ip-api.com/json/?fields=status,countryCode")
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: serde_json::Value = resp.json().await.ok()?;
    if body.get("status").and_then(|s| s.as_str()) != Some("success") {
        return None;
    }
    body.get("countryCode")
        .and_then(|c| c.as_str())
        .map(|c| c.to_uppercase())
}

/// Reads all real signals — OS-provided (timezone, locale) plus one live IP
/// geolocation call. Never returns a fabricated value — every field is
/// either something actually read/observed or None. Async solely because of
/// the IP call; callers already run inside a Tokio runtime (config load
/// happens inside `#[tokio::main]`), so this awaits directly rather than
/// spawning a blocking reqwest client (which panics if constructed from
/// inside an already-running runtime).
pub async fn detect() -> HardwareRegionSignals {
    let timezone_name = read_system_timezone_name();
    let timezone_country = timezone_name
        .as_ref()
        .and_then(|tz| timezone_country_table().get(tz).cloned());
    let (locale_raw, locale_country) = read_locale_country();
    let ip_country = detect_ip_country().await;

    HardwareRegionSignals {
        timezone_name,
        timezone_country,
        locale_raw,
        ip_country,
        locale_country,
    }
}
