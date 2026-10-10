//! SHARED BOUNDED DISPATCH — the one concurrency primitive for fan-out
//! model-call sites (docs/EFFICIENCY_GUIDE.md §6a).
//!
//! Three sites each invented their own wave-barrier loop (grammar pass,
//! cross-reference, AMT lanes) and two real ordering-bug classes were found
//! across those reinventions (the grammar pass's silent result discard, the
//! AMT lanes' misattribution). This helper exists so no site re-derives
//! those mechanics again:
//!
//! - **Index-carrying by construction**: every task returns `(index,
//!   result)`; results land in a pre-sized slot Vec at that index. A
//!   completion-order mismatch is structurally impossible.
//! - **Panic-safe by elimination**: a panicked task produces no return
//!   value (carried index included), but its slot stays `None` after the
//!   full drain — by construction, exactly the failed item — and is filled
//!   with a loud `Err`, never silently dropped.
//! - **Continuously-fed, not wave-barrier**: a bounded semaphore gates
//!   dispatch, so a straggler in one item does not stall the next batch —
//!   width is the only concurrency knob, and slots free the moment their
//!   own item finishes.
//! - **Per-item timeout is the CALLER's contract, inside its future**,
//!   sized to that item's real work (a single call ≠ a multi-candidate
//!   fallback walk — the flat-timeout mistake is how the lanes gap
//!   happened). The helper only bounds concurrency and guarantees
//!   attribution; it imposes no time semantics of its own.

use tokio::sync::Semaphore;
use std::future::Future;
use std::sync::Arc;

/// Dispatch `items` through `f` with at most `width` futures in flight,
/// returning one result per item in INPUT order. `f` must be cloneable and
/// 'static (it moves its item's owned work into the spawned future); its
/// future should apply whatever timeout that item's real work requires.
/// A panicked item's slot is filled with a loud `Err`, never silently
/// dropped; every other item's result lands at its own index.
pub async fn dispatch_bounded<I, V, Fut>(
    items: Vec<I>,
    width: usize,
    f: impl Fn(I) -> Fut + Send + Clone + 'static,
) -> Vec<Result<V, String>>
where
    I: Send + 'static,
    V: Send + 'static,
    Fut: Future<Output = Result<V, String>> + Send + 'static,
{
    let total = items.len();
    if total == 0 {
        return Vec::new();
    }
    let width = width.max(1);
    let semaphore = Arc::new(Semaphore::new(width));
    let mut set = tokio::task::JoinSet::new();

    for (idx, item) in items.into_iter().enumerate() {
        let semaphore = semaphore.clone();
        let f = f.clone();
        set.spawn(async move {
            // The permit is held for the item's whole future — continuous
            // dispatch: the next item starts the moment ANY slot frees.
            let _permit = semaphore
                .acquire()
                .await
                .expect("dispatch semaphore closed while in use");
            (idx, f(item).await)
        });
    }

    let mut slots: Vec<Option<Result<V, String>>> = (0..total).map(|_| None).collect();
    while let Some(res) = set.join_next().await {
        match res {
            // JoinError here = the TASK panicked (the item's own Err is inside
            // the Ok tuple). No index survives a panic — the slot stays None
            // and is filled by elimination below.
            Ok((idx, r)) => {
                if idx < slots.len() {
                    slots[idx] = Some(r);
                }
            }
            Err(join_err) => {
                tracing::warn!(
                    completed = slots.iter().filter(|s| s.is_some()).count(),
                    total,
                    error = %join_err,
                    "dispatch: item task panicked — its slot is filled as a loud item failure by elimination"
                );
            }
        }
    }

    slots
        .into_iter()
        .enumerate()
        .map(|(idx, slot)| {
            slot.unwrap_or_else(|| {
                tracing::warn!(
                    idx,
                    "dispatch: item produced no result — recorded as a loud item failure"
                );
                Err("item task panicked (no result produced)".to_string())
            })
        })
        .collect()
}

/// Hardware-scaled default width for I/O-bound fan-out work (1× cores —
/// conservative first, raise with ledger evidence per the efficiency
/// guide's sizing rule). Callers with a specific concurrency contract pass
/// their own width; this is the shared default.
pub fn default_width() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        .max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn results_land_at_their_own_indices() {
        // Deliberately uneven completion: item 0 sleeps longest but must
        // land at slot 0 — the completion-order bug class made structural.
        let items: Vec<(u64, u64)> = vec![(0, 30), (1, 1), (2, 5)];
        let out = dispatch_bounded(items, 3, |(idx, ms)| async move {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
            Ok(idx)
        })
        .await;
        let ids: Vec<u64> = out.into_iter().map(|r| r.unwrap()).collect();
        assert_eq!(ids, vec![0, 1, 2]);
    }

    #[tokio::test]
    async fn width_bounds_concurrent_flight() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let flight = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let items: Vec<()> = vec![(); 8];
        let f_flight = flight.clone();
        let f_peak = peak.clone();
        let out = dispatch_bounded(items, 2, move |()| {
            let flight = f_flight.clone();
            let peak = f_peak.clone();
            async move {
                let now = flight.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                flight.fetch_sub(1, Ordering::SeqCst);
                Ok(())
            }
        })
        .await;
        assert!(out.iter().all(|r| r.is_ok()));
        assert!(peak.load(Ordering::SeqCst) <= 2, "width bound exceeded");
    }

    #[tokio::test]
    async fn panicked_item_becomes_a_loud_err_slot() {
        let items: Vec<u64> = vec![0, 1, 2];
        let out = dispatch_bounded(items, 3, |idx| async move {
            if idx == 1 {
                panic!("item panicked — no index survives");
            }
            Ok(idx)
        })
        .await;
        assert!(out[0].is_ok());
        assert_eq!(
            out[1].as_ref().unwrap_err(),
            "item task panicked (no result produced)"
        );
        assert!(out[2].is_ok());
    }

    #[tokio::test]
    async fn empty_items_returns_empty() {
        let out: Vec<Result<u64, String>> =
            dispatch_bounded(Vec::new(), 3, |(): ()| async { Ok(0) }).await;
        assert!(out.is_empty());
    }
}
