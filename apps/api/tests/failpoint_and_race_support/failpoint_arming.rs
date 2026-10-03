//! Arming helpers over `api_failpoints` and the check of an injected answer.
//!
//! **Role:** names the three ways a case arms a point (`fail`, `fail_once`, `pause`) as methods of
//! the held suite lock, bundles a pause with its guard, and checks the `500` a request answers
//! when an armed point fails it.
//! **Position:** used by the `failure_injection*` and `controlled_races*` suites; re-exports the
//! registry surface (`lock_suite`, `Failpoint`, `CATALOGUE`, `reach`, …) so a suite imports it
//! from one place.
//! **Signals & state:** none of its own; every guard lives in the process-global registry until it
//! drops.
//! **Invariants:** a guard borrows the suite lock, so no point stays armed after the lock drops; a
//! `pause` holds only the first arrival and releases it when its [`PausedFailpoint`] drops.

use axum::http::StatusCode;
use serde_json::Value;

pub use api_failpoints::{
    ArmGuard, CATALOGUE, FailAction, Failpoint, FailpointSuiteLock, PauseHandle, lock_suite, reach,
};

/// The arming verbs of a held suite lock.
///
/// ```ignore
/// let suite = lock_suite().await;
/// let guard = suite.fail(Failpoint::SessionLogoutBeforeCommit);
/// assert_eq!(guard.arrivals(), 0);
/// ```
pub trait FailpointArming {
    /// Every arrival at `point` fails until the guard drops.
    fn fail(&self, point: Failpoint) -> ArmGuard<'_>;
    /// The first arrival at `point` fails; later arrivals pass.
    fn fail_once(&self, point: Failpoint) -> ArmGuard<'_>;
    /// The first arrival at `point` waits there until [`PausedFailpoint::release`] or the drop of
    /// the returned value; later arrivals pass.
    fn pause(&self, point: Failpoint) -> PausedFailpoint<'_>;
}

impl FailpointArming for FailpointSuiteLock {
    fn fail(&self, point: Failpoint) -> ArmGuard<'_> {
        self.arm(point, FailAction::Fail)
    }

    fn fail_once(&self, point: Failpoint) -> ArmGuard<'_> {
        self.arm(point, FailAction::FailOnce)
    }

    fn pause(&self, point: Failpoint) -> PausedFailpoint<'_> {
        let handle = PauseHandle::new();
        let guard = self.arm(point, FailAction::Pause(handle.clone()));
        PausedFailpoint { guard, handle }
    }
}

/// A point armed with a pause: its guard and the handle that observes and releases the held
/// arrival.
pub struct PausedFailpoint<'suite> {
    guard: ArmGuard<'suite>,
    handle: PauseHandle,
}

impl PausedFailpoint<'_> {
    /// Resolves once the first arrival is held at the point; panics after
    /// `api_failpoints::PAUSE_REACH_BOUND`.
    pub async fn reached(&self) {
        self.handle.reached().await;
    }

    /// Lets the held arrival continue.
    pub fn release(&self) {
        self.handle.release();
    }

    /// The pause handle itself, for a task that waits on it elsewhere.
    pub fn handle(&self) -> &PauseHandle {
        &self.handle
    }

    /// How many requests reached the point since it was armed.
    pub fn arrivals(&self) -> usize {
        self.guard.arrivals()
    }
}

/// Asserts the answer of a request an armed point failed on an `ApiError` path: `500`, the error
/// `internal error`, and `details.failpoint` naming `point`.
pub fn assert_injected_failure(status: StatusCode, body: &Value, point: Failpoint) {
    assert_eq!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{} must answer 500; body {body}",
        point.name()
    );
    assert_eq!(body["error"], "internal error", "body {body}");
    assert_eq!(
        body["details"]["failpoint"],
        point.name(),
        "the answer must name the failpoint that failed it; body {body}"
    );
}
