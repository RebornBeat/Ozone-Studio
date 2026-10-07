//! PIPELINE ADMISSION GATE — universal order applied to pipeline calls
//! (operator, 2026-10-05: "we should never be skipping — it should all be
//! properly ordered").
//!
//! Replaces the hard reject at `executor.rs`'s concurrency cap (11th call
//! got an error and the caller's work was lost — the AMT branch lanes
//! swallowed it silently). Nothing is rejected here: every call is
//! ENQUEUED in the order it arrived and executes when a slot frees.
//!
//! Two-tier ordering (universal order: user work first, system work
//! fills in behind):
//!   `User`  — orchestrate steps, gates, anything a request is waiting on
//!   `Lane`  — AMT batch lanes (parallel, but below user work)
//!   `Loop`  — background loop calls (I-loop, assistant, meta, amt_loop)
//! Within a tier: strict FIFO. User tier always drains before Lane, Lane
//! before Loop.
//!
//! Safety: this gate REPLACES nothing safety-critical — the adapter and
//! metered watchdogs still bound every call once it executes; the gate
//! only orders the START. Queue depth and wait times are real metrics
//! (take_snapshot) for the monitor — never fabricated.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Instant;

/// Who is asking to run a pipeline. Order of the enum = priority order
/// (User drains first).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CallPriority {
    User,
    Lane,
    Loop,
}

struct Waiter {
    priority: CallPriority,
    seq: u64,
    enqueued_at: Instant,
    tx: tokio::sync::oneshot::Sender<()>,
}

/// The admission gate. One per process (the executor holds it).
pub struct OrderedPipelineGate {
    max_active: usize,
    /// Waiters older than this are granted past strict tier order (see the
    /// starvation-guard notes). Per-gate so tests can shorten it without
    /// leaking into parallel tests through a process-global.
    age_grant_ms: u64,
    inner: Mutex<GateInner>,
    next_seq: std::sync::atomic::AtomicU64,
}

struct GateInner {
    active: usize,
    /// FIFO per tier; User drained before Lane, Lane before Loop.
    waiters: [VecDeque<Waiter>; 3],
    total_enqueued: u64,
    total_executed: u64,
    total_wait_ms_peak: u64,
    /// Grants made past max_active by the starvation guard (see
    /// force_grant_aged) — real metrics, never folded into "executed".
    total_overflow_grants: u64,
}

/// A waiter queued longer than this is granted on the next slot OR by the
/// starvation guard, whichever comes first. Rationale: (a) tier priority
/// must not become indefinite starvation for Loop/Lane work under sustained
/// user traffic; (b) a nested/remote dispatch that calls back into the host
/// while holding its own ticket can deadlock a fully-saturated gate — the
/// callback sits in the queue and NO release() can ever fire. The guard's
/// grant breaks that circular wait from outside; overflow is bounded by the
/// number of aged waiters (the nesting depth), reported in the snapshot.
const AGE_GRANT_DEFAULT_MS: u64 = 90_000;

fn tier_index(p: CallPriority) -> usize {
    match p {
        CallPriority::User => 0,
        CallPriority::Lane => 1,
        CallPriority::Loop => 2,
    }
}

impl OrderedPipelineGate {
    pub fn new(max_active: usize) -> Self {
        Self {
            max_active: max_active.max(1),
            age_grant_ms: AGE_GRANT_DEFAULT_MS,
            inner: Mutex::new(GateInner {
                active: 0,
                waiters: [VecDeque::new(), VecDeque::new(), VecDeque::new()],
                total_enqueued: 0,
                total_executed: 0,
                total_wait_ms_peak: 0,
                total_overflow_grants: 0,
            }),
            next_seq: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Enqueue the call and await its turn. ALWAYS resolves — nothing is
    /// rejected or skipped; the only delay is the wait for a slot in
    /// proper order.
    pub async fn admit(&self, priority: CallPriority) -> AdmissionTicket {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let seq = self
            .next_seq
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut granted = false;
        {
            let mut g = self.inner.lock().unwrap();
            g.total_enqueued += 1;
            if g.active < self.max_active && g.waiters.iter().all(|q| q.is_empty()) {
                g.active += 1;
                g.total_executed += 1;
                granted = true;
            } else {
                let t = tier_index(priority);
                g.waiters[t].push_back(Waiter {
                    priority,
                    seq,
                    enqueued_at: Instant::now(),
                    tx,
                });
            }
        }
        if !granted {
            let _ = rx.await; // released by release() — lost-wakeup-proof:
                              // the sender exists in the queue before we
                              // await, so the ordering can never race.
        }
        AdmissionTicket {
            enqueued_at: Instant::now(),
            gate: self,
        }
    }

    /// Release a slot and wake the next waiter in proper order (User →
    /// Lane → Loop, FIFO within tier).
    pub fn release(&self) {
        let mut guard = self.inner.lock().unwrap();
        let g = &mut *guard;
        g.active = g.active.saturating_sub(1);
        // STARVATION BOUND: a waiter older than AGE_GRANT_AFTER is granted
        // ahead of the tier order (still oldest-first among aged). Under any
        // turnover faster than the threshold, tier priority is untouched;
        // past it, no tier starves and no nested-callback chain stays
        // circularly blocked.
        let mut aged: Option<(usize, usize)> = None; // (tier, position)
        let mut oldest: Option<Instant> = None;
        for (t, tier) in g.waiters.iter().enumerate() {
            for (pos, w) in tier.iter().enumerate() {
                if w.enqueued_at.elapsed() >= self.age_grant()
                    && oldest.map_or(true, |o| w.enqueued_at < o)
                {
                    aged = Some((t, pos));
                    oldest = Some(w.enqueued_at);
                }
            }
        }
        if let Some((t, pos)) = aged {
            if let Some(w) = g.waiters[t].remove(pos) {
                if !w.tx.is_closed() {
                    g.active += 1;
                    g.total_executed += 1;
                    g.total_overflow_grants += 1;
                    let _ = w.tx.send(());
                    return;
                }
            }
        }
        // Drain in priority order. A waiter whose caller was cancelled while queued
        // has a closed sender; granting it would take a slot no ticket will ever return.
        for tier in g.waiters.iter_mut() {
            while let Some(w) = tier.pop_front() {
                if w.tx.is_closed() {
                    continue;
                }
                g.active += 1;
                g.total_executed += 1;
                let wait = w.enqueued_at.elapsed().as_millis() as u64;
                if wait > g.total_wait_ms_peak {
                    g.total_wait_ms_peak = wait;
                }
                let _ = w.tx.send(());
                return;
            }
        }
    }

    /// Grant every waiter older than AGE_GRANT_AFTER, even past max_active
    /// (overflow). Called by the gate's own keeper thread: this is the ONLY
    /// path that can break a fully-saturated nested-callback deadlock, where
    /// every ticket is held by a call waiting on a queued callback and no
    /// release() can ever fire. Returns how many were granted.
    pub fn force_grant_aged(&self) -> usize {
        let mut granted = 0usize;
        let mut guard = self.inner.lock().unwrap();
        let g = &mut *guard;
        for t in 0..3 {
            let mut pos = 0;
            while pos < g.waiters[t].len() {
                let aged = g.waiters[t][pos].enqueued_at.elapsed() >= self.age_grant();
                if !aged {
                    pos += 1;
                    continue;
                }
                let w = g.waiters[t].remove(pos).unwrap();
                if w.tx.is_closed() {
                    continue; // cancelled while queued: no slot accounting
                }
                g.active += 1;
                g.total_executed += 1;
                g.total_overflow_grants += 1;
                granted += 1;
                let _ = w.tx.send(());
                // do not advance pos: the next element shifted into it
            }
        }
        if granted > 0 {
            tracing::warn!(
                granted,
                active = g.active,
                max_active = self.max_active,
                "gate starvation guard granted aged waiter(s) past max_active — sustained saturation or a nested-callback deadlock just broke"
            );
        }
        granted
    }

    fn age_grant(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.age_grant_ms)
    }

    /// TEST-ONLY constructor with a shortened aging threshold.
    #[cfg(test)]
    fn new_with_age_grant(max_active: usize, age_grant_ms: u64) -> Self {
        let mut g = Self::new(max_active);
        g.age_grant_ms = age_grant_ms;
        g
    }

    /// Spawn the starvation-guard keeper: every 5s it grants waiters older
    /// than the aging threshold even past max_active. This is the only
    /// enforcer that can break a fully-saturated nested-callback deadlock
    /// (every ticket held by a call waiting on a queued callback — no
    /// release() can ever fire). Exits when the gate is dropped.
    pub fn spawn_keeper(self: &std::sync::Arc<Self>) {
        let weak = std::sync::Arc::downgrade(self);
        let spawned = std::thread::Builder::new()
            .name("gate-starvation-keeper".into())
            .spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(5));
                let Some(gate) = weak.upgrade() else { return };
                gate.force_grant_aged();
                drop(gate);
            });
        if spawned.is_err() {
            tracing::warn!("gate starvation keeper failed to spawn — aged waiters still get grants on every release(), but a fully-saturated nested deadlock would need operator intervention");
        }
    }

    /// Real gate metrics for the monitor — measured, never estimated.
    pub fn snapshot(&self) -> GateSnapshot {
        let g = self.inner.lock().unwrap();
        GateSnapshot {
            active: g.active,
            max_active: self.max_active,
            queued: [
                g.waiters[0].len(),
                g.waiters[1].len(),
                g.waiters[2].len(),
            ],
            total_enqueued: g.total_enqueued,
            total_executed: g.total_executed,
            peak_wait_ms: g.total_wait_ms_peak,
            total_overflow_grants: g.total_overflow_grants,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GateSnapshot {
    pub active: usize,
    pub max_active: usize,
    /// Queued per tier: [user, lane, loop]
    pub queued: [usize; 3],
    pub total_enqueued: u64,
    pub total_executed: u64,
    pub peak_wait_ms: u64,
    /// Grants past max_active by the starvation guard.
    pub total_overflow_grants: u64,
}

/// Held from grant until the caller finishes; releasing it hands the slot
/// to the next waiter in order. Forgetting to release would wedge the gate,
/// so the ticket releases on Drop.
pub struct AdmissionTicket<'a> {
    enqueued_at: Instant,
    gate: &'a OrderedPipelineGate,
}

impl AdmissionTicket<'_> {
    pub fn wait_ms_so_far(&self) -> u64 {
        self.enqueued_at.elapsed().as_millis() as u64
    }
}

impl Drop for AdmissionTicket<'_> {
    fn drop(&mut self) {
        self.gate.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grants_immediately_when_slots_free() {
        let gate = OrderedPipelineGate::new(2);
        let _a = futures_test_block(&gate, CallPriority::User);
        let _b = futures_test_block(&gate, CallPriority::Lane);
        let snap = gate.snapshot();
        assert_eq!(snap.active, 2);
        assert_eq!(snap.queued, [0, 0, 0]);
    }

    // Tiny sync helper — the gate's admit is async only because of the
    // oneshot await; under no contention it completes without parking.
    fn futures_test_block<'a>(
        gate: &'a OrderedPipelineGate,
        p: CallPriority,
    ) -> AdmissionTicket<'a> {
        tokio_test_block_on(gate.admit(p))
    }

    fn tokio_test_block_on<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(f)
    }

    #[tokio::test]
    async fn user_tier_drains_before_loop() {
        let gate = std::sync::Arc::new(OrderedPipelineGate::new(1));
        let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::<&'static str>::new()));
        let held = gate.admit(CallPriority::User).await;

        let (g, o) = (gate.clone(), order.clone());
        let loop_h = tokio::spawn(async move {
            let _t = g.admit(CallPriority::Loop).await;
            o.lock().unwrap().push("loop");
        });
        while gate.snapshot().queued[2] < 1 {
            tokio::task::yield_now().await;
        }
        let (g, o) = (gate.clone(), order.clone());
        let user_h = tokio::spawn(async move {
            let _t = g.admit(CallPriority::User).await;
            o.lock().unwrap().push("user");
        });
        while gate.snapshot().queued[0] < 1 {
            tokio::task::yield_now().await;
        }

        drop(held);
        loop_h.await.unwrap();
        user_h.await.unwrap();
        assert_eq!(*order.lock().unwrap(), vec!["user", "loop"]);
    }

    #[tokio::test]
    async fn starvation_guard_grants_without_release() {
        let gate = std::sync::Arc::new(OrderedPipelineGate::new_with_age_grant(1, 0));
        let held = gate.admit(CallPriority::User).await;

        let g = gate.clone();
        let waiter = tokio::spawn(async move {
            let _t = g.admit(CallPriority::Loop).await;
            true
        });
        while gate.snapshot().queued[2] < 1 {
            tokio::task::yield_now().await;
        }
        // No release() ever fires — the guard must break the wait.
        let granted = gate.force_grant_aged();
        assert_eq!(granted, 1, "aged waiter must be force-granted");
        // Assert BEFORE awaiting the waiter: its ticket drops when the task
        // finishes, and the test task must observe the overflow state first.
        let snap = gate.snapshot();
        assert_eq!(snap.active, 2, "overflow grant past max_active");
        assert_eq!(snap.total_overflow_grants, 1);
        assert!(waiter.await.unwrap(), "force-granted waiter must proceed");
        drop(held); // both tickets drop → release twice → active 0
    }

    #[tokio::test]
    async fn cancelled_waiter_does_not_leak_a_slot() {
        let gate = std::sync::Arc::new(OrderedPipelineGate::new(1));
        let held = gate.admit(CallPriority::User).await;

        let g = gate.clone();
        let waiter = tokio::spawn(async move {
            let _t = g.admit(CallPriority::Loop).await;
        });
        while gate.snapshot().queued[2] < 1 {
            tokio::task::yield_now().await;
        }
        waiter.abort();
        let _ = waiter.await;

        drop(held);
        let granted = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            gate.admit(CallPriority::User),
        )
        .await;
        assert!(granted.is_ok(), "a cancelled waiter consumed the only slot");
    }
}

tokio::task_local! {
    static CALL_PRIORITY: CallPriority;
}

/// Tags the pipeline calls made inside `fut` with an origin tier. Untagged calls are User.
pub fn with_priority<F: std::future::Future>(
    priority: CallPriority,
    fut: F,
) -> impl std::future::Future<Output = F::Output> {
    CALL_PRIORITY.scope(priority, fut)
}

pub fn current_call_priority() -> CallPriority {
    CALL_PRIORITY.try_with(|p| *p).unwrap_or(CallPriority::User)
}
