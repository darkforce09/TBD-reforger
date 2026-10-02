//! Unit coverage for the pause handle on its own: a new handle is neither reached nor released,
//! the held arrival marks it reached and waits for the release, a release before the arrival lets
//! it pass at once, releasing twice is harmless, and waiting for an arrival that never comes
//! fails the case.

use std::time::Duration;

use tokio::time::{sleep, timeout};

use super::*;

/// How long a case waits to show a held arrival is still waiting.
const PENDING_WINDOW: Duration = Duration::from_millis(50);

/// The bound on an arrival or a wait that must resolve.
const RESOLVE_BOUND: Duration = Duration::from_secs(5);

#[test]
fn failpoints_pause_handle_starts_neither_reached_nor_released() {
    let pause = PauseHandle::new();
    assert!(!pause.is_reached());
    assert!(!pause.is_released());
    assert!(!PauseHandle::default().is_reached());
}

#[tokio::test]
async fn failpoints_pause_handle_holds_the_arrival_until_released() {
    let pause = PauseHandle::new();
    let arrival = pause.clone();
    let held = tokio::spawn(async move { arrival.hold().await });
    pause.reached_within(RESOLVE_BOUND).await;
    assert!(pause.is_reached(), "every clone sees the arrival");
    sleep(PENDING_WINDOW).await;
    assert!(!held.is_finished(), "the arrival waits for the release");
    pause.release();
    timeout(RESOLVE_BOUND, held)
        .await
        .expect("a released arrival goes on")
        .expect("the held task joins");
    assert!(pause.is_released());
}

#[tokio::test]
async fn failpoints_pause_handle_released_before_the_arrival_lets_it_pass_at_once() {
    let pause = PauseHandle::new();
    pause.release();
    pause.release();
    timeout(RESOLVE_BOUND, pause.hold())
        .await
        .expect("an arrival after the release does not wait");
    assert!(pause.is_reached());
    assert!(pause.is_released());
}

#[tokio::test]
#[should_panic(expected = "no request reached the paused failpoint")]
async fn failpoints_pause_handle_fails_the_case_when_no_request_arrives() {
    PauseHandle::new().reached_within(PENDING_WINDOW).await;
}
