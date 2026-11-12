//! Native history store — ONE git-like version history for BOTH workspace files (`file` scope, written by the file beacon) and ZSEI
//! container records (`container` scope, written by the integrity monitor before every update/delete).
//!
//! Operator 2026-11-12: "reconstruct via diff, takes less space than complete backups, make both work together".
//!
//! Layout under `<root>` (default `<OZONE_ZSEI_DATA_DIR|zsei_data>/history`):
//!   objects/<hh>/<blake3 hex>      one content-defined chunk, stored ONCE (a chunk shared by 50 versions, or by a file and a container,
//!                                  exists once) — this is the "diff" storage: a new version only adds the chunks that actually changed
//!   versions/<scope>/<id>.jsonl    append-only version rows {v, ts, reason, path?, size, hash, chunks[]}; one row per DISTINCT content
//!
//! A version is rebuilt by concatenating its chunks (every chunk and the whole are blake3-verified on read): O(size), no delta chains,
//! no keyframes. Differences between versions are computed on demand (git-mcp). History is never pruned, rotated or capped (no-caps
//! doctrine); de-duplication is what bounds growth. Chunk sizes below are CHUNKING parameters (the system's only size notion), not caps
//! on what may be stored — any size of input is split, never refused.
//!
//! Durability: every object is written temp + fsync + rename; every index row is appended and fsynced. A failed write is returned to the
//! caller, who retries (integrity: `retry_until_ok`; beacon: forgets its baseline so the next poll retries). Nothing is skipped.
//!
//! Erasure (jurisdiction): history is append-only, so an erasure of personal data must reach this store too — see
//! `docs/evidence/jurisdiction-deepening/` gap G10; `scopes()` + `list()` + `read_version()` give the erasure planner what it needs.

pub mod diff;

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use serde::{Deserialize, Serialize};

/// Smallest chunk the chunker will cut (bytes) — except the final tail of an input.
const CHUNK_MIN: usize = 2048;
/// Cut mask: a boundary where `hash & MASK == 0` gives ~8 KiB average chunks.
const CHUNK_MASK: u64 = (1 << 13) - 1;
/// Longest chunk (bytes): forces a boundary in boundary-free data (e.g. long runs). A chunking parameter, not a size cap on inputs.
const CHUNK_MAX: usize = 64 * 1024;

/// One recorded version of one scoped item.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VersionRow {
    pub v: u64,
    /// Unix milliseconds the content was observed / captured.
    pub ts: u64,
    pub reason: String,
    #[serde(default)]
    pub path: Option<String>,
    pub size: u64,
    /// blake3 hex of the WHOLE content.
    pub hash: String,
    /// blake3 hex of each chunk, in order.
    pub chunks: Vec<String>,
}

pub fn default_root() -> PathBuf {
    let dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    Path::new(&dir).join("history")
}

fn gear() -> &'static [u64; 256] {
    static GEAR: OnceLock<[u64; 256]> = OnceLock::new();
    GEAR.get_or_init(|| {
        // splitmix64 from a fixed seed: deterministic table, so identical content always chunks identically across boots.
        let mut t = [0u64; 256];
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        for slot in t.iter_mut() {
            x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = x;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            *slot = z ^ (z >> 31);
        }
        t
    })
}

/// Content-defined chunk boundaries `(start, end)` covering `data` exactly.
pub fn chunk_bounds(data: &[u8]) -> Vec<(usize, usize)> {
    let g = gear();
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut h: u64 = 0;
    for (i, b) in data.iter().enumerate() {
        h = (h << 1).wrapping_add(g[*b as usize]);
        let len = i + 1 - start;
        if (len >= CHUNK_MIN && (h & CHUNK_MASK) == 0) || len >= CHUNK_MAX {
            out.push((start, i + 1));
            start = i + 1;
            h = 0;
        }
    }
    if start < data.len() {
        out.push((start, data.len()));
    }
    out
}

fn hex_of(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

fn bad(msg: impl Into<String>) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, msg.into())
}

fn valid_part(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub struct History {
    root: PathBuf,
    /// Serialises writers and caches each item's latest row (None = known empty).
    latest: Mutex<HashMap<String, Option<VersionRow>>>,
    /// Recorded path -> file-scope id (lazily built from disk, kept current by `record_with`): lets a walk ask "what changed in THIS file".
    by_path: Mutex<Option<HashMap<String, String>>>,
}

impl History {
    pub fn open(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into(), latest: Mutex::new(HashMap::new()), by_path: Mutex::new(None) }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn index_path(&self, scope: &str, id: &str) -> PathBuf {
        self.root.join("versions").join(scope).join(format!("{id}.jsonl"))
    }

    fn object_path(&self, hex: &str) -> PathBuf {
        self.root.join("objects").join(&hex[..2]).join(hex)
    }

    fn check(scope: &str, id: &str) -> std::io::Result<()> {
        if !valid_part(scope) || !valid_part(id) {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "history: scope/id must be [A-Za-z0-9_-]+"));
        }
        Ok(())
    }

    fn read_rows(&self, scope: &str, id: &str) -> Vec<VersionRow> {
        std::fs::read_to_string(self.index_path(scope, id))
            .map(|t| t.lines().filter_map(|l| serde_json::from_str::<VersionRow>(l).ok()).collect())
            .unwrap_or_default()
    }

    /// Every recorded version of one item, oldest first.
    pub fn list(&self, scope: &str, id: &str) -> std::io::Result<Vec<VersionRow>> {
        Self::check(scope, id)?;
        Ok(self.read_rows(scope, id))
    }

    pub fn latest(&self, scope: &str, id: &str) -> std::io::Result<Option<VersionRow>> {
        Self::check(scope, id)?;
        let mut g = self.latest.lock().unwrap_or_else(|p| p.into_inner());
        let key = format!("{scope}/{id}");
        if let Some(v) = g.get(&key) {
            return Ok(v.clone());
        }
        let last = self.read_rows(scope, id).pop();
        g.insert(key, last.clone());
        Ok(last)
    }

    /// `(scope, id, versions)` for every item that has history.
    pub fn scopes(&self) -> Vec<(String, String, usize)> {
        let mut out = Vec::new();
        let base = self.root.join("versions");
        let Ok(scopes) = std::fs::read_dir(&base) else { return out };
        for s in scopes.filter_map(|e| e.ok()) {
            let scope = s.file_name().to_string_lossy().to_string();
            let Ok(files) = std::fs::read_dir(s.path()) else { continue };
            for f in files.filter_map(|e| e.ok()) {
                let name = f.file_name().to_string_lossy().to_string();
                if let Some(id) = name.strip_suffix(".jsonl") {
                    let n = std::fs::read_to_string(f.path()).map(|t| t.lines().filter(|l| !l.trim().is_empty()).count()).unwrap_or(0);
                    out.push((scope.clone(), id.to_string(), n));
                }
            }
        }
        out.sort();
        out
    }

    /// Record `bytes` as the newest version of `scope/id` at `ts`. Identical to the latest version = no new row (that row is returned).
    pub fn record_with(&self, scope: &str, id: &str, bytes: &[u8], reason: &str, path: Option<&str>, ts: u64) -> std::io::Result<VersionRow> {
        Self::check(scope, id)?;
        let key = format!("{scope}/{id}");
        let mut g = self.latest.lock().unwrap_or_else(|p| p.into_inner());
        let latest = match g.get(&key) {
            Some(v) => v.clone(),
            None => {
                let l = self.read_rows(scope, id).pop();
                g.insert(key.clone(), l.clone());
                l
            }
        };
        let hash = hex_of(bytes);
        if let Some(l) = &latest {
            if l.hash == hash {
                return Ok(l.clone());
            }
        }
        let mut chunks = Vec::new();
        for (s, e) in chunk_bounds(bytes) {
            let part = &bytes[s..e];
            let ch = hex_of(part);
            let p = self.object_path(&ch);
            if !p.exists() {
                if let Some(parent) = p.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let tmp = p.with_extension("tmp");
                {
                    let mut f = std::fs::File::create(&tmp)?;
                    f.write_all(part)?;
                    f.sync_all()?;
                }
                std::fs::rename(&tmp, &p)?;
            }
            chunks.push(ch);
        }
        let row = VersionRow {
            v: latest.as_ref().map(|l| l.v + 1).unwrap_or(1),
            ts,
            reason: reason.to_string(),
            path: path.map(|s| s.to_string()),
            size: bytes.len() as u64,
            hash,
            chunks,
        };
        let idx = self.index_path(scope, id);
        if let Some(parent) = idx.parent() {
            std::fs::create_dir_all(parent)?;
        }
        {
            let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&idx)?;
            let line = serde_json::to_string(&row).map_err(|e| bad(e.to_string()))?;
            f.write_all(format!("{line}\n").as_bytes())?;
            f.sync_data()?;
        }
        g.insert(key, Some(row.clone()));
        if scope == "file" {
            if let Some(p) = path {
                let mut bp = self.by_path.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(map) = bp.as_mut() {
                    map.insert(p.to_string(), id.to_string());
                }
            }
        }
        Ok(row)
    }

    pub fn record(&self, scope: &str, id: &str, bytes: &[u8], reason: &str, path: Option<&str>) -> std::io::Result<VersionRow> {
        self.record_with(scope, id, bytes, reason, path, now_ms())
    }

    /// The file-scope id recorded for a path (the newest recorded path wins), if any.
    pub fn id_for_path(&self, path: &str) -> Option<String> {
        let mut bp = self.by_path.lock().unwrap_or_else(|e| e.into_inner());
        if bp.is_none() {
            let mut map: HashMap<String, String> = HashMap::new();
            let mut items: Vec<(u64, String, String)> = Vec::new();
            for (scope, id, _) in self.scopes() {
                if scope != "file" {
                    continue;
                }
                if let Some(last) = self.read_rows(&scope, &id).into_iter().rev().find(|r| r.path.is_some()) {
                    items.push((last.ts, id, last.path.unwrap_or_default()));
                }
            }
            items.sort();
            for (_, id, p) in items {
                map.insert(p, id);
            }
            *bp = Some(map);
        }
        bp.as_ref().and_then(|m| m.get(path).cloned())
    }

    /// One line per file that has history: its newest version, when, why, size, and the line delta against the version before it. This is the
    /// "what changed" context a walk gets for the files it reads. No window and no cap: the NEWEST change is always shown, whatever its age.
    pub fn changes_block(&self, paths: &[String]) -> Option<String> {
        let mut lines: Vec<String> = Vec::new();
        for path in paths {
            let Some(id) = self.id_for_path(path) else { continue };
            let rows = self.read_rows("file", &id);
            let Some(last) = rows.last() else { continue };
            let mut line = format!("- {path}: version {} of {} (recorded {} ms since epoch, {}, {} bytes)", last.v, rows.len(), last.ts, last.reason, last.size);
            if rows.len() >= 2 {
                let prev = &rows[rows.len() - 2];
                if let (Ok(a), Ok(b)) = (self.rebuild(prev), self.rebuild(last)) {
                    let d = diff::diff_versions(&a, &b, &prev.chunks, &last.chunks);
                    if d["binary"].as_bool() == Some(true) {
                        line.push_str(&format!("; binary, {} chunk(s) added, {} removed vs the previous version", d["chunks_added"], d["chunks_removed"]));
                    } else {
                        let (mut add, mut del) = (0usize, 0usize);
                        for h in d["hunks"].as_array().cloned().unwrap_or_default() {
                            for l in h["lines"].as_array().cloned().unwrap_or_default() {
                                match l.as_str().and_then(|s| s.chars().next()) {
                                    Some('+') => add += 1,
                                    Some('-') => del += 1,
                                    _ => {}
                                }
                            }
                        }
                        line.push_str(&format!("; +{add} / -{del} lines vs the previous version"));
                    }
                }
            } else {
                line.push_str("; first recorded version (baseline)");
            }
            lines.push(line);
        }
        if lines.is_empty() {
            None
        } else {
            Some(lines.join("\n"))
        }
    }

    /// Rebuild version `v` (every chunk and the whole are blake3-verified).
    pub fn read_version(&self, scope: &str, id: &str, v: u64) -> std::io::Result<Vec<u8>> {
        Self::check(scope, id)?;
        let row = self
            .read_rows(scope, id)
            .into_iter()
            .find(|r| r.v == v)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, format!("history: {scope}/{id} has no version {v}")))?;
        self.rebuild(&row)
    }

    pub fn rebuild(&self, row: &VersionRow) -> std::io::Result<Vec<u8>> {
        let mut out = Vec::with_capacity(row.size as usize);
        for ch in &row.chunks {
            if ch.len() < 3 || !ch.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(bad(format!("history: malformed chunk id in version {}", row.v)));
            }
            let part = std::fs::read(self.object_path(ch))?;
            if hex_of(&part) != *ch {
                return Err(bad(format!("history: chunk {ch} fails its hash (corruption)")));
            }
            out.extend_from_slice(&part);
        }
        if hex_of(&out) != row.hash {
            return Err(bad(format!("history: version {} fails its whole-content hash", row.v)));
        }
        Ok(out)
    }

    /// Verify the newest version of an item (integrity check). `Ok(None)` = no history.
    pub fn verify_latest(&self, scope: &str, id: &str) -> std::io::Result<Option<u64>> {
        match self.latest(scope, id)? {
            None => Ok(None),
            Some(row) => self.rebuild(&row).map(|_| Some(row.v)),
        }
    }

    /// Restore a workspace file's bytes from version `v`: the CURRENT file is recorded first (reason `pre-restore`), so a restore is itself
    /// reversible. Returns the path written.
    pub fn restore_file(&self, id: &str, v: u64) -> std::io::Result<PathBuf> {
        let rows = self.list("file", id)?;
        let target = rows.iter().find(|r| r.v == v).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no such version"))?;
        let path = target
            .path
            .clone()
            .or_else(|| rows.iter().rev().find_map(|r| r.path.clone()))
            .ok_or_else(|| bad("history: version has no recorded path"))?;
        let bytes = self.rebuild(target)?;
        if let Ok(current) = std::fs::read(&path) {
            self.record("file", id, &current, "pre-restore", Some(&path))?;
        }
        let tmp = PathBuf::from(format!("{path}.ozone-restore.tmp"));
        {
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(&bytes)?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp, &path)?;
        Ok(PathBuf::from(path))
    }

    /// Import the pre-2026-11-12 stores WITHOUT deleting them: integrity `integrity/rollback/<id>_<v>.snapshot` (container scope) and the
    /// beacon's `file_history/` (file scope). Idempotent (an already-imported legacy row is recognised by scope/id/ts/reason). Any error is
    /// returned and the marker is NOT written, so the next boot retries — nothing is skipped.
    pub fn migrate_legacy(&self, data_dir: &Path) -> std::io::Result<usize> {
        let marker = self.root.join(".legacy_imported_v1");
        if marker.exists() {
            return Ok(0);
        }
        let mut imported = 0usize;
        // integrity snapshots
        let rb = data_dir.join("integrity").join("rollback");
        if let Ok(rd) = std::fs::read_dir(&rb) {
            let mut items: Vec<(u64, u64, PathBuf)> = Vec::new();
            for e in rd.filter_map(|e| e.ok()) {
                let name = e.file_name().to_string_lossy().to_string();
                let Some(stem) = name.strip_suffix(".snapshot") else { continue };
                let Some((id, ver)) = stem.split_once('_') else { continue };
                if let (Ok(id), Ok(ver)) = (id.parse::<u64>(), ver.parse::<u64>()) {
                    items.push((id, ver, e.path()));
                }
            }
            items.sort();
            for (id, _ver, p) in items {
                let ts = std::fs::metadata(&p)
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let ids = id.to_string();
                if self.read_rows("container", &ids).iter().any(|r| r.ts == ts && r.reason == "legacy-integrity") {
                    continue;
                }
                let bytes = std::fs::read(&p)?;
                self.record_with("container", &ids, &bytes, "legacy-integrity", None, ts)?;
                imported += 1;
            }
        }
        // beacon history
        let fh = data_dir.join("file_history");
        if let Ok(rd) = std::fs::read_dir(fh.join("snapshots")) {
            for d in rd.filter_map(|e| e.ok()) {
                let cid = d.file_name().to_string_lossy().to_string();
                if !valid_part(&cid) {
                    continue;
                }
                let path: Option<String> = std::fs::read_to_string(fh.join(format!("{cid}.jsonl")))
                    .ok()
                    .and_then(|t| t.lines().filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok()).filter_map(|v| v["path"].as_str().map(|s| s.to_string())).last());
                let mut snaps: Vec<(u64, PathBuf)> = Vec::new();
                if let Ok(files) = std::fs::read_dir(d.path()) {
                    for f in files.filter_map(|e| e.ok()) {
                        let name = f.file_name().to_string_lossy().to_string();
                        if let Some(stem) = name.strip_suffix(".snap") {
                            if let Ok(ms) = stem.split('_').next().unwrap_or("").parse::<u64>() {
                                snaps.push((ms, f.path()));
                            }
                        }
                    }
                }
                snaps.sort();
                for (ms, p) in snaps {
                    if self.read_rows("file", &cid).iter().any(|r| r.ts == ms && r.reason == "legacy-beacon") {
                        continue;
                    }
                    let bytes = std::fs::read(&p)?;
                    self.record_with("file", &cid, &bytes, "legacy-beacon", path.as_deref(), ms)?;
                    imported += 1;
                }
            }
        }
        std::fs::create_dir_all(&self.root)?;
        std::fs::write(&marker, format!("{imported}\n"))?;
        Ok(imported)
    }
}

/// One shared `History` per root (the beacon, the integrity monitor and the restore route must serialise on the same instance).
pub fn open_shared(root: impl Into<PathBuf>) -> Arc<History> {
    static REG: OnceLock<Mutex<HashMap<PathBuf, Arc<History>>>> = OnceLock::new();
    let root: PathBuf = root.into();
    let mut g = REG.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap_or_else(|p| p.into_inner());
    if let Some(h) = g.get(&root) {
        return h.clone();
    }
    let h = Arc::new(History::open(root.clone()));
    // Legacy import runs once, at first open of the DEFAULT layout (root = <data>/history).
    if root.file_name().map(|n| n == "history").unwrap_or(false) {
        if let Some(data_dir) = root.parent() {
            match h.migrate_legacy(data_dir) {
                Ok(n) if n > 0 => tracing::info!(imported = n, "history: legacy integrity/beacon snapshots imported (originals kept)"),
                Ok(_) => {}
                Err(e) => tracing::error!(error = %e, "history: legacy import failed — it will be retried at next boot; originals untouched"),
            }
        }
    }
    g.insert(root, h.clone());
    h
}

/// The shared history at the default root.
pub fn shared() -> Arc<History> {
    open_shared(default_root())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("ozone_history_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn chunks_cover_input_exactly() {
        let data: Vec<u8> = (0..300_000u32).map(|i| (i.wrapping_mul(2654435761) >> 24) as u8).collect();
        let b = chunk_bounds(&data);
        assert_eq!(b.first().unwrap().0, 0);
        assert_eq!(b.last().unwrap().1, data.len());
        for w in b.windows(2) {
            assert_eq!(w[0].1, w[1].0);
        }
    }

    #[test]
    fn roundtrip_dedupe_and_unchanged_is_no_row() {
        let root = tmp("rt");
        let h = History::open(&root);
        let a: Vec<u8> = (0..200_000u32).map(|i| (i.wrapping_mul(2654435761) >> 24) as u8).collect();
        let r1 = h.record("container", "7", &a, "t", None).unwrap();
        let r1b = h.record("container", "7", &a, "t", None).unwrap();
        assert_eq!(r1.v, r1b.v);
        let mut b = a.clone();
        b[100_000] ^= 0xFF;
        let r2 = h.record("container", "7", &b, "t", None).unwrap();
        assert_eq!(r2.v, 2);
        // the edit touches one chunk: most chunk ids are shared between versions
        let shared = r2.chunks.iter().filter(|c| r1.chunks.contains(c)).count();
        assert!(shared + 2 >= r2.chunks.len());
        assert_eq!(h.read_version("container", "7", 1).unwrap(), a);
        assert_eq!(h.read_version("container", "7", 2).unwrap(), b);
        assert!(h.record("../x", "7", &a, "t", None).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn corruption_is_detected() {
        let root = tmp("cor");
        let h = History::open(&root);
        let a = b"hello history".to_vec();
        let r = h.record("file", "1", &a, "t", Some("/x")).unwrap();
        std::fs::write(h.object_path(&r.chunks[0]), b"tampered").unwrap();
        assert!(h.read_version("file", "1", 1).is_err());
        assert!(h.verify_latest("file", "1").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_content_roundtrips() {
        let root = tmp("empty");
        let h = History::open(&root);
        h.record("file", "2", b"", "t", None).unwrap();
        assert_eq!(h.read_version("file", "2", 1).unwrap(), Vec::<u8>::new());
        let _ = std::fs::remove_dir_all(&root);
    }
}
