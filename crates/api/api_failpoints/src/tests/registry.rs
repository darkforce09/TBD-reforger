//! Unit coverage for the registry: unarmed points are inert, the arrival rule of each action, the
//! arrival count, disarming and releasing on drop, a second arming refused without disturbing the
//! first, and the suite lock admitting one case at a time.
//!
//! Every case holds the suite lock and arms only the two unit-test points, so no case disturbs
//! another unit test of the library that passes a catalogue point.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;

use tokio::time::{sleep, timeout};

use super::*;
use crate::CATALOGUE;

/// How long a case waits to show a held arrival or a blocked lock is still waiting.
const PENDING_WINDOW: Duration = Duration::from_millis(50);

/// The bound on an arrival or a lock that must resolve.
const RESOLVE_BOUND: Duration = Duration::from_secs(5);

const FIRST: Failpoint = Failpoint::RegistryUnitTestFirst;
const SECOND: Failpoint = Failpoint::RegistryUnitTestSecond;

/// The failure an armed `Fail` or `FailOnce` returns at `failpoint`.
fn injected(failpoint: Failpoint) -> Result<()> {
    Err(Error::InjectedFailure { failpoint })
}

#[tokio::test]
async fn failpoints_unarmed_points_are_inert() {
    let _suite = lock_suite().await;
    for failpoint in CATALOGUE.into_iter().chain([FIRST, SECOND]) {
        assert_eq!(
            reach(failpoint).await,
            Ok(()),
            "{} is inert until armed",
            failpoint.name()
        );
    }
}

#[tokio::test]
async fn failpoints_fail_fails_every_arrival_until_its_guard_drops() {
    let suite = lock_suite().await;
    let guard = suite.arm(FIRST, FailAction::Fail);
    assert_eq!(reach(FIRST).await, injected(FIRST));
    assert_eq!(reach(FIRST).await, injected(FIRST));
    assert_eq!(
        reach(SECOND).await,
        Ok(()),
        "arming one point leaves the others inert"
    );
    assert_eq!(guard.arrivals(), 2);
    drop(guard);
    assert_eq!(
        reach(FIRST).await,
        Ok(()),
        "a dropped guard disarms the point"
    );
}

#[tokio::test]
async fn failpoints_fail_once_fails_only_the_first_arrival() {
    let suite = lock_suite().await;
    let guard = suite.arm(FIRST, FailAction::FailOnce);
    assert_eq!(reach(FIRST).await, injected(FIRST));
    assert_eq!(reach(FIRST).await, Ok(()));
    assert_eq!(reach(FIRST).await, Ok(()));
    assert_eq!(guard.arrivals(), 3);
}

#[tokio::test]
async fn failpoints_pause_holds_the_first_arrival_until_released() {
    let suite = lock_suite().await;
    let pause = PauseHandle::new();
    let guard = suite.arm(FIRST, FailAction::Pause(pause.clone()));
    let held = tokio::spawn(reach(FIRST));
    pause.reached().await;
    sleep(PENDING_WINDOW).await;
    assert!(
        !held.is_finished(),
        "the first arrival waits for the release"
    );
    assert_eq!(
        reach(FIRST).await,
        Ok(()),
        "a later arrival passes while the first is held"
    );
    assert_eq!(guard.arrivals(), 2);
    pause.release();
    let outcome = timeout(RESOLVE_BOUND, held)
        .await
        .expect("a released arrival goes on")
        .expect("the held task joins");
    assert_eq!(outcome, Ok(()));
}

#[tokio::test]
async fn failpoints_dropping_the_guard_releases_a_held_arrival() {
    let suite = lock_suite().await;
    let pause = PauseHandle::new();
    let guard = suite.arm(FIRST, FailAction::Pause(pause.clone()));
    let held = tokio::spawn(reach(FIRST));
    pause.reached().await;
    drop(guard);
    assert!(pause.is_released());
    let outcome = timeout(RESOLVE_BOUND, held)
        .await
        .expect("the arrival goes on once its guard drops")
        .expect("the held task joins");
    assert_eq!(outcome, Ok(()));
}

#[tokio::test]
async fn failpoints_arming_an_armed_point_panics_and_keeps_the_first_arming() {
    let suite = lock_suite().await;
    let first_arming = suite.arm(FIRST, FailAction::FailOnce);
    let second_arming = catch_unwind(AssertUnwindSafe(|| suite.arm(FIRST, FailAction::Fail)));
    assert!(
        second_arming.is_err(),
        "a second arming of one point panics"
    );
    assert_eq!(
        reach(FIRST).await,
        injected(FIRST),
        "the first arming still applies"
    );
    assert_eq!(reach(FIRST).await, Ok(()));
    assert_eq!(first_arming.arrivals(), 2);
    let other_point = suite.arm(SECOND, FailAction::Fail);
    assert_eq!(
        reach(SECOND).await,
        injected(SECOND),
        "the registry is not poisoned"
    );
    drop(other_point);
}

#[tokio::test]
async fn failpoints_suite_lock_admits_one_case_at_a_time() {
    let suite = lock_suite().await;
    let contender = tokio::spawn(async {
        let _suite = lock_suite().await;
    });
    sleep(PENDING_WINDOW).await;
    assert!(!contender.is_finished(), "a second case waits for the lock");
    drop(suite);
    timeout(RESOLVE_BOUND, contender)
        .await
        .expect("the lock passes on once released")
        .expect("the contender joins");
}
