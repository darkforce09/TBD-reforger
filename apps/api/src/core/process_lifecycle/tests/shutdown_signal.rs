//! Unit coverage for the shutdown signal: not begun until begun, every waiter woken, late waiters
//! resolved at once, idempotent beginning, and a dropped signal that can no longer begin.
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
async fn a_new_signal_has_not_begun_and_its_waiters_stay_pending() {
    let signal = ShutdownSignal::new();
    assert!(!signal.has_begun());
    assert!(
        timeout(PENDING_WINDOW, signal.begun()).await.is_err(),
        "a waiter on a signal that has not begun stays pending"
    );
    assert!(!ShutdownSignal::default().has_begun());
}

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

#[tokio::test]
async fn beginning_twice_is_beginning_once() {
    let signal = ShutdownSignal::new();
    let early = signal.begun();
    signal.begin();
    signal.begin();
    assert!(signal.has_begun(), "a begun signal stays begun");
    timeout(RESOLVE_BOUND, early)
        .await
        .expect("a waiter from before both begins resolves");
    assert_eq!(signal.begun().now_or_never(), Some(()));
}

#[tokio::test]
async fn a_signal_dropped_before_it_begins_leaves_its_waiters_pending() {
    let signal = ShutdownSignal::new();
    let waiter = signal.begun();
    drop(signal);
    assert!(
        timeout(PENDING_WINDOW, waiter).await.is_err(),
        "a signal that can no longer begin never resolves its waiters"
    );
}

#[test]
fn the_process_shutdown_is_one_shared_instance() {
    assert!(
        std::ptr::eq(process_shutdown(), process_shutdown()),
        "every caller sees the same process-wide signal"
    );
}
