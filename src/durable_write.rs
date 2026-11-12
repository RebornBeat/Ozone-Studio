//! DURABLE BOOKKEEPING WRITES (operator 2026-11-11: "there is no skip — every failure is fixed or the task pauses and retries").
//!
//! Hit logs, judgment logs, walk metrics, call spans, ledgers, snapshots and stores used to be written "best-effort": one attempt,
//! a warning, and the row was gone. This helper keeps the exact call shape those sites already had (ONE synchronous attempt on the
//! caller — no added latency on the hot path) and changes what happens on failure:
//!
//!   * the job goes onto a per-path pending queue and a single background worker retries it until it SUCCEEDS (2 s, doubling,
//!     held at 60 s between attempts — spacing only, never a give-up);
//!   * a path with pending jobs keeps its order: later jobs for that path queue behind the failed one (appends stay in order);
//!   * nothing is capped or dropped. The only thing that bounds the queue is the host's own memory, like every other in-memory
//!     structure here; hit / judgment logs are NEVER rotated or truncated by this helper;
//!   * failure is VISIBLE: `tracing::warn!` on the first failure and on recovery, a `durable_write_stalled` /
//!     `durable_write_recovered` graph event, and counters from [`stats`] (pending, retries, written, last error per path);
//!   * a retried append after a failed attempt starts with a newline so a half-written tail can never glue to the next row
//!     (readers skip blank lines and unreadable rows, and none of these files is rewritten from this side — shared files are
//!     appended to by other processes too, so no rollback-truncate is attempted);
//!   * `replace` (whole-file snapshots: stores, reports, derived stats) carries a sequence number; a queued older snapshot is
//!     discarded only when a NEWER snapshot of the same path has already been applied — a newer whole file supersedes an older
//!     one, which is not a dropped write.
//!
//! What it does NOT do: survive the process dying while a job is still pending (the queue is in memory; a dead disk cannot hold
//! its own retry journal). [`flush_for`] is called on graceful shutdown to drain what it can and reports what is still pending.

use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, Once, OnceLock};
use std::time::{Duration, Instant};

const FIRST_DELAY: Duration = Duration::from_secs(2);
const HELD_DELAY: Duration = Duration::from_secs(60);

enum Body {
    Append(String),
    Replace(Arc<Vec<u8>>),
}

struct Job {
    seq: u64,
    body: Body,
}

struct PathQ {
    jobs: VecDeque<Job>,
    inflight: bool,
    delay: Duration,
    next_try: Instant,
    failing: bool,
    /// A previous attempt on this path failed: the next append starts with a newline.
    dirty: bool,
    applied_seq: u64,
    /// The newest whole-file snapshot not yet applied, so a reader in this process sees it ([`read_to_string`]).
    latest: Option<(u64, Arc<Vec<u8>>)>,
    last_error: String,
    retries: u64,
}

impl PathQ {
    fn new() -> Self {
        PathQ {
            jobs: VecDeque::new(),
            inflight: false,
            delay: FIRST_DELAY,
            next_try: Instant::now(),
            failing: false,
            dirty: false,
            applied_seq: 0,
            latest: None,
            last_error: String::new(),
            retries: 0,
        }
    }
}

struct Shared {
    queues: Mutex<HashMap<PathBuf, PathQ>>,
    /// Wakes the worker (new work, or the earliest retry time moved).
    wake: Condvar,
    /// Wakes `flush_for` callers (the queues drained).
    idle: Condvar,
}

static SEQ: AtomicU64 = AtomicU64::new(1);
static WRITTEN: AtomicU64 = AtomicU64::new(0);
static RETRIES: AtomicU64 = AtomicU64::new(0);
static SUPERSEDED: AtomicU64 = AtomicU64::new(0);

fn shared() -> &'static Shared {
    static S: OnceLock<Shared> = OnceLock::new();
    let s = S.get_or_init(|| Shared { queues: Mutex::new(HashMap::new()), wake: Condvar::new(), idle: Condvar::new() });
    static START: Once = Once::new();
    START.call_once(|| {
        if let Err(e) = std::thread::Builder::new().name("durable-write".into()).spawn(worker) {
            // The worker could not start: every failed write would then sit unretried. Say so loudly; the synchronous first
            // attempt on each caller still works.
            tracing::error!(error = %e, "durable_write worker thread did not start; failed writes will not be retried");
        }
    });
    s
}

fn lock(s: &Shared) -> std::sync::MutexGuard<'_, HashMap<PathBuf, PathQ>> {
    s.queues.lock().unwrap_or_else(|e| e.into_inner())
}

/// Append one line (a trailing newline is added when missing) to `path`. One synchronous attempt now; a failure is retried in
/// order until it succeeds.
pub fn append(path: impl Into<PathBuf>, line: impl Into<String>) {
    submit(path.into(), Body::Append(line.into()));
}

/// Append one JSON value as a line.
pub fn append_json(path: impl Into<PathBuf>, value: &serde_json::Value) {
    append(path, value.to_string());
}

/// Replace the whole file at `path` (write temp + rename). One synchronous attempt now; a failure is retried until a snapshot at
/// least this new has been applied.
pub fn replace(path: impl Into<PathBuf>, bytes: impl Into<Vec<u8>>) {
    submit(path.into(), Body::Replace(Arc::new(bytes.into())));
}

fn submit(path: PathBuf, body: Body) {
    let s = shared();
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    {
        let mut g = lock(s);
        let q = g.entry(path.clone()).or_insert_with(PathQ::new);
        if !q.jobs.is_empty() || q.inflight {
            // Order is kept per path: queue behind what is already waiting.
            if let Body::Replace(b) = &body {
                q.latest = Some((seq, b.clone()));
            }
            push(q, Job { seq, body });
            s.wake.notify_all();
            return;
        }
    }
    match run(&path, &body, false) {
        Ok(()) => {
            WRITTEN.fetch_add(1, Ordering::Relaxed);
            if matches!(body, Body::Replace(_)) {
                let mut g = lock(s);
                let q = g.entry(path).or_insert_with(PathQ::new);
                q.applied_seq = q.applied_seq.max(seq);
                clear_applied_latest(q);
            }
        }
        Err(e) => {
            let mut g = lock(s);
            let q = g.entry(path.clone()).or_insert_with(PathQ::new);
            if matches!(body, Body::Replace(_)) && seq < q.applied_seq {
                SUPERSEDED.fetch_add(1, Ordering::Relaxed);
                return;
            }
            q.dirty = true;
            note_failure(&path, q, &e);
            q.next_try = Instant::now() + q.delay;
            if let Body::Replace(b) = &body {
                q.latest = Some((seq, b.clone()));
            }
            push(q, Job { seq, body });
            s.wake.notify_all();
        }
    }
}

fn clear_applied_latest(q: &mut PathQ) {
    if let Some((ls, _)) = &q.latest {
        if *ls <= q.applied_seq {
            q.latest = None;
        }
    }
}

/// Read a file the way the rest of this process should see it: a whole-file snapshot written through [`replace`] that has not reached
/// disk yet (its write is being retried) is returned in place of the older bytes on disk, so a read-modify-write caller never works
/// from a stale copy and loses the unwritten change.
pub fn read_to_string(path: impl AsRef<Path>) -> std::io::Result<String> {
    let path = path.as_ref();
    {
        let g = lock(shared());
        if let Some((_, bytes)) = g.get(path).and_then(|q| q.latest.as_ref()) {
            return Ok(String::from_utf8_lossy(bytes).into_owned());
        }
    }
    std::fs::read_to_string(path)
}

/// Queue a job, collapsing consecutive whole-file snapshots (only the newest of two queued `Replace` jobs matters).
fn push(q: &mut PathQ, job: Job) {
    if matches!(job.body, Body::Replace(_)) {
        if let Some(back) = q.jobs.back_mut() {
            if matches!(back.body, Body::Replace(_)) {
                *back = job;
                SUPERSEDED.fetch_add(1, Ordering::Relaxed);
                return;
            }
        }
    }
    q.jobs.push_back(job);
}

fn note_failure(path: &Path, q: &mut PathQ, e: &std::io::Error) {
    q.last_error = e.to_string();
    if !q.failing {
        q.failing = true;
        tracing::warn!(path = %path.display(), error = %e, "durable write failed; retrying until it succeeds (nothing is dropped)");
        crate::graph_events::emit(
            "durable_write_stalled",
            0,
            0,
            format!("durable_write:{}", path.display()),
            "durable_write",
            vec![e.to_string()],
        );
    }
}

fn run(path: &Path, body: &Body, dirty: bool) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)?;
        }
    }
    match body {
        Body::Append(line) => {
            // ONE write call so concurrent appenders (O_APPEND) cannot interleave a row body and its newline.
            let mut buf = String::with_capacity(line.len() + 2);
            if dirty {
                buf.push('\n');
            }
            buf.push_str(line);
            if !line.ends_with('\n') {
                buf.push('\n');
            }
            let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
            f.write_all(buf.as_bytes())
        }
        Body::Replace(bytes) => {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".to_string());
            let tmp = path.with_file_name(format!("{name}.dw{}", std::process::id()));
            std::fs::write(&tmp, bytes.as_slice())?;
            std::fs::rename(&tmp, path)
        }
    }
}

fn worker() {
    let s = shared();
    loop {
        let mut g = lock(s);
        let now = Instant::now();
        let ready = g.iter().find(|(_, q)| !q.jobs.is_empty() && !q.inflight && q.next_try <= now).map(|(p, _)| p.clone());
        let Some(path) = ready else {
            if g.values().all(|q| q.jobs.is_empty() && !q.inflight) {
                s.idle.notify_all();
                drop(s.wake.wait(g).unwrap_or_else(|e| e.into_inner()));
            } else {
                let earliest = g.values().filter(|q| !q.jobs.is_empty() && !q.inflight).map(|q| q.next_try).min();
                let wait = earliest.map(|t| t.saturating_duration_since(now)).unwrap_or(FIRST_DELAY);
                drop(s.wake.wait_timeout(g, wait).unwrap_or_else(|e| e.into_inner()).0);
            }
            continue;
        };
        // Take the front job out; it goes back at the front if the attempt fails.
        let (job, dirty) = {
            let q = g.get_mut(&path).expect("queue exists for a ready path");
            let Some(job) = q.jobs.pop_front() else { continue };
            q.inflight = true;
            (job, q.dirty)
        };
        let superseded = matches!(job.body, Body::Replace(_)) && g.get(&path).map(|q| job.seq < q.applied_seq).unwrap_or(false);
        drop(g);

        let res = if superseded { Ok(()) } else { run(&path, &job.body, dirty) };

        let mut g = lock(s);
        let Some(q) = g.get_mut(&path) else { continue };
        q.inflight = false;
        match res {
            Ok(()) => {
                if superseded {
                    SUPERSEDED.fetch_add(1, Ordering::Relaxed);
                } else {
                    WRITTEN.fetch_add(1, Ordering::Relaxed);
                    if matches!(job.body, Body::Replace(_)) {
                        q.applied_seq = q.applied_seq.max(job.seq);
                    }
                }
                clear_applied_latest(q);
                q.delay = FIRST_DELAY;
                q.dirty = false;
                if q.failing && q.jobs.is_empty() {
                    q.failing = false;
                    tracing::info!(path = %path.display(), retries = q.retries, "durable write recovered; pending rows written");
                    crate::graph_events::emit(
                        "durable_write_recovered",
                        0,
                        0,
                        format!("durable_write:{}", path.display()),
                        "durable_write",
                        vec![format!("{} retries", q.retries)],
                    );
                    q.retries = 0;
                }
                q.next_try = Instant::now();
            }
            Err(e) => {
                q.retries += 1;
                RETRIES.fetch_add(1, Ordering::Relaxed);
                q.dirty = true;
                note_failure(&path, q, &e);
                q.delay = (q.delay * 2).min(HELD_DELAY);
                q.next_try = Instant::now() + q.delay;
                q.jobs.push_front(job);
            }
        }
        if g.values().all(|q| q.jobs.is_empty() && !q.inflight) {
            s.idle.notify_all();
        }
    }
}

/// Run a fallible async store/IO operation until it SUCCEEDS (2 s, doubling, held at 60 s between attempts): the no-skip form of
/// "log loudly and carry on". The caller PAUSES at this point (nothing after it runs on a failure), the failure is visible (a warning
/// on the first failure, a `durable_write_stalled` graph event, and a recovery line), and it resumes when the operation works. Use it
/// for idempotent writes/reads whose result the next step needs.
pub async fn retry_until_ok<T, E, F, Fut>(label: &str, mut attempt: F) -> T
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut delay = FIRST_DELAY;
    let mut failed = false;
    loop {
        match attempt().await {
            Ok(v) => {
                if failed {
                    tracing::info!(label, "operation recovered after retries");
                }
                return v;
            }
            Err(e) => {
                if !failed {
                    failed = true;
                    tracing::warn!(label, error = %e, "operation failed; pausing and retrying until it succeeds (nothing is skipped)");
                    crate::graph_events::emit("durable_write_stalled", 0, 0, format!("retry:{label}"), "durable_write", vec![e.to_string()]);
                }
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(HELD_DELAY);
            }
        }
    }
}

/// Number of jobs still waiting (queued or in flight).
pub fn pending() -> usize {
    let g = lock(shared());
    g.values().map(|q| q.jobs.len() + usize::from(q.inflight)).sum()
}

/// Counters and per-path state for health / monitor surfaces. Real values only.
pub fn stats() -> serde_json::Value {
    let g = lock(shared());
    let paths: Vec<serde_json::Value> = g
        .iter()
        .filter(|(_, q)| !q.jobs.is_empty() || q.inflight || q.failing)
        .map(|(p, q)| {
            serde_json::json!({
                "path": p.display().to_string(),
                "pending": q.jobs.len() + usize::from(q.inflight),
                "failing": q.failing,
                "last_error": q.last_error,
            })
        })
        .collect();
    serde_json::json!({
        "pending": g.values().map(|q| q.jobs.len() + usize::from(q.inflight)).sum::<usize>(),
        "written": WRITTEN.load(Ordering::Relaxed),
        "retries": RETRIES.load(Ordering::Relaxed),
        "superseded_snapshots": SUPERSEDED.load(Ordering::Relaxed),
        "stalled_paths": paths,
    })
}

/// Wait up to `timeout` for the queues to drain (graceful shutdown). Returns how many jobs are STILL pending — the caller logs it.
pub fn flush_for(timeout: Duration) -> usize {
    let s = shared();
    let deadline = Instant::now() + timeout;
    let mut g = lock(s);
    s.wake.notify_all();
    loop {
        let left: usize = g.values().map(|q| q.jobs.len() + usize::from(q.inflight)).sum();
        if left == 0 {
            return 0;
        }
        let now = Instant::now();
        if now >= deadline {
            return left;
        }
        // Make every waiting path eligible now: a shutdown drain does not wait out a retry spacing.
        for q in g.values_mut() {
            q.next_try = now;
        }
        s.wake.notify_all();
        g = s.idle.wait_timeout(g, deadline - now).unwrap_or_else(|e| e.into_inner()).0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("durable_write_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn append_and_replace_write_immediately() {
        let d = tmpdir("basic");
        let log = d.join("a.jsonl");
        append(&log, "one");
        append(&log, "two\n");
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "one\ntwo\n");
        let snap = d.join("s.json");
        replace(&snap, b"v1".to_vec());
        replace(&snap, b"v2".to_vec());
        assert_eq!(std::fs::read_to_string(&snap).unwrap(), "v2");
    }

    #[test]
    fn failed_append_is_retried_in_order_until_it_succeeds() {
        let d = tmpdir("retry");
        // `blocker` is a FILE, so a log under it cannot be created: the first attempts fail.
        let blocker = d.join("blocker");
        std::fs::write(&blocker, b"x").unwrap();
        let log = blocker.join("sub").join("x.jsonl");
        append(&log, "first");
        append(&log, "second");
        assert!(pending() >= 2);
        // Fix the cause; the worker must write both rows, in order, without anyone calling anything again.
        std::fs::remove_file(&blocker).unwrap();
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline && std::fs::read_to_string(&log).map(|t| t.matches('\n').count() < 2).unwrap_or(true) {
            std::thread::sleep(Duration::from_millis(100));
        }
        let text = std::fs::read_to_string(&log).unwrap();
        let rows: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(rows, vec!["first", "second"]);
    }

    #[test]
    fn older_snapshot_never_overwrites_a_newer_applied_one() {
        let d = tmpdir("seq");
        let snap = d.join("s.json");
        let q = |p: &Path| {
            let g = lock(shared());
            g.get(p).map(|q| q.applied_seq).unwrap_or(0)
        };
        replace(&snap, b"old".to_vec());
        replace(&snap, b"new".to_vec());
        assert!(q(&snap) > 0);
        assert_eq!(std::fs::read_to_string(&snap).unwrap(), "new");
    }
}
