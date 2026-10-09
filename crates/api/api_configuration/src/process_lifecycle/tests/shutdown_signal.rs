//! Unit coverage for the shutdown signal: every waiter woken, late waiters resolved at once.
//!
//! No case begins [`process_shutdown`]: the library's unit tests share one process, and a begun
//! process shutdown ends every event stream opened after it.

use std::time::Duration;

use futures::FutureExt;
use tokio::time::timeout;

use super::*;

/// How long a case waits to show a waiter is still pending.
const PENDING_WINDOW: Duration = Duration::from_millis(50);

/// The bound on a waiter that must resolve.
const RESOLVE_BOUND: Duration = Duration::from_secs(1);

#[tokio::test]
async fn begin_wakes_every_waiter_created_before_it() {
    let signal = ShutdownSignal::new();
    let waiters: Vec<_> = (0..3).map(|_| tokio::spawn(signal.begun())).collect();
    tokio::time::sleep(PENDING_WINDOW).await;
    assert!(
        waiters.iter().all(|waiter| !waiter.is_finished()),
        "no waiter resolves before the signal begins"
    );

    signal.begin();

    assert!(signal.has_begun());
    for waiter in waiters {
        timeout(RESOLVE_BOUND, waiter)
            .await
            .expect("every waiter resolves once the signal begins")
            .expect("the waiter task completes");
    }
}

#[tokio::test]
async fn a_waiter_created_after_begin_resolves_on_its_first_poll() {
    let signal = ShutdownSignal::new();
    signal.begin();
    assert_eq!(
        signal.begun().now_or_never(),
        Some(()),
        "a late waiter needs no wake-up"
    );
}
