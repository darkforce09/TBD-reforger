//! The process-global registry of armed failpoints, the suite lock that arms them, and the check
//! every call site runs.
//!
//! **Role:** [`lock_suite`] serialises the cases of one test process that arm failpoints;
//! [`FailpointSuiteLock::arm`] arms one point with a [`FailAction`] and returns the [`ArmGuard`]
//! that disarms it; [`reach`] is what the `fail_point!` macro runs at a call site.
//! **Position:** `api_failpoints`, compiled only with the `failpoints` feature; the
//! `failure_injection*` and `controlled_races*` suites and this module's unit tests arm; call sites
//! in the domains reach.
//! **Signals & state:** `ARMED`, a `std::sync::Mutex` over the armed entries (each with its action
//! and its arrival count), never held across an `.await`; `SUITE_LOCK`, a `tokio::sync::Mutex`
//! held for the whole of an arming case.
//! **Invariants:** a point with no entry is inert and [`reach`] answers `Ok(())`; arming needs a
//! held [`FailpointSuiteLock`], so two cases of one process never arm at the same time; a point
//! is armed at most once at a time (arming it again panics, without poisoning `ARMED`); `Fail`
//! fails every arrival, `FailOnce` the first only, and `Pause` holds the first only, every later
//! arrival passing; dropping the [`ArmGuard`] removes the entry and releases a `Pause`, so a
//! held request never outlives its case.

use std::marker::PhantomData;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::catalogue::Failpoint;
use crate::error::{Error, Result};
use crate::pause_handle::PauseHandle;

/// Every armed point, at most one entry per [`Failpoint`].
static ARMED: Mutex<Vec<ArmedEntry>> = Mutex::new(Vec::new());

/// The lock one arming case holds from before it arms until after its guards drop.
static SUITE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// What an armed failpoint does to the requests that reach it.
#[derive(Clone, Debug)]
pub enum FailAction {
    /// Every arrival returns an [`Error::InjectedFailure`].
    Fail,
    /// The first arrival returns an [`Error::InjectedFailure`]; later arrivals pass.
    FailOnce,
    /// The first arrival waits at the point until the handle is released; later arrivals pass.
    Pause(PauseHandle),
}

/// One armed point: its action and how many requests have reached it since it was armed.
struct ArmedEntry {
    failpoint: Failpoint,
    action: FailAction,
    arrivals: usize,
}

/// What [`reach`] does for one arrival, decided under the registry lock and carried out after it.
enum ArrivalOutcome {
    Pass,
    Fail,
    Hold(PauseHandle),
}

/// The suite lock of this process, held by the one case that may arm failpoints.
///
/// Every case of a `failure_injection*` or `controlled_races*` binary takes it first, because the
/// registry is global to the process and the binary's cases run in parallel threads.
pub struct FailpointSuiteLock {
    _held: tokio::sync::MutexGuard<'static, ()>,
}

/// Waits until no other case of this process holds the suite lock, then holds it.
pub async fn lock_suite() -> FailpointSuiteLock {
    FailpointSuiteLock {
        _held: SUITE_LOCK.lock().await,
    }
}

impl FailpointSuiteLock {
    /// Arms `failpoint` with `action` until the returned guard drops; the guard borrows the lock,
    /// so it cannot outlive it.
    ///
    /// # Panics
    ///
    /// When `failpoint` is already armed.
    pub fn arm(&self, failpoint: Failpoint, action: FailAction) -> ArmGuard<'_> {
        let release_on_drop = match &action {
            FailAction::Pause(handle) => Some(handle.clone()),
            FailAction::Fail | FailAction::FailOnce => None,
        };
        let already_armed = {
            let mut armed = armed_entries();
            let already_armed = armed.iter().any(|entry| entry.failpoint == failpoint);
            if !already_armed {
                armed.push(ArmedEntry {
                    failpoint,
                    action,
                    arrivals: 0,
                });
            }
            already_armed
        };
        assert!(
            !already_armed,
            "failpoint {} is already armed; drop its ArmGuard before arming it again",
            failpoint.name()
        );
        ArmGuard {
            failpoint,
            release_on_drop,
            _suite: PhantomData,
        }
    }
}

/// Keeps one failpoint armed; dropping it disarms the point and releases a `Pause` it holds.
#[must_use = "the failpoint disarms as soon as its guard drops"]
pub struct ArmGuard<'suite> {
    failpoint: Failpoint,
    release_on_drop: Option<PauseHandle>,
    _suite: PhantomData<&'suite FailpointSuiteLock>,
}

impl ArmGuard<'_> {
    /// How many requests have reached the point since it was armed.
    pub fn arrivals(&self) -> usize {
        armed_entries()
            .iter()
            .find(|entry| entry.failpoint == self.failpoint)
            .map_or(0, |entry| entry.arrivals)
    }
}

impl Drop for ArmGuard<'_> {
    fn drop(&mut self) {
        armed_entries().retain(|entry| entry.failpoint != self.failpoint);
        if let Some(handle) = &self.release_on_drop {
            handle.release();
        }
    }
}

/// Passes `failpoint`: `Ok(())` when it is not armed or the action lets this arrival pass, an
/// [`Error::InjectedFailure`] when a `Fail` or `FailOnce` fails it, and, for the arrival a `Pause`
/// holds, `Ok(())` once the pause is released.
pub async fn reach(failpoint: Failpoint) -> Result<()> {
    match record_arrival(failpoint) {
        ArrivalOutcome::Pass => Ok(()),
        ArrivalOutcome::Fail => Err(Error::InjectedFailure { failpoint }),
        ArrivalOutcome::Hold(handle) => {
            handle.hold().await;
            Ok(())
        }
    }
}

/// Counts one arrival at `failpoint` and decides its outcome, holding the registry lock only for
/// the decision.
fn record_arrival(failpoint: Failpoint) -> ArrivalOutcome {
    let mut armed = armed_entries();
    let Some(entry) = armed.iter_mut().find(|entry| entry.failpoint == failpoint) else {
        return ArrivalOutcome::Pass;
    };
    entry.arrivals += 1;
    let first_arrival = entry.arrivals == 1;
    match &entry.action {
        FailAction::Fail => ArrivalOutcome::Fail,
        FailAction::FailOnce if first_arrival => ArrivalOutcome::Fail,
        FailAction::Pause(handle) if first_arrival => ArrivalOutcome::Hold(handle.clone()),
        FailAction::FailOnce | FailAction::Pause(_) => ArrivalOutcome::Pass,
    }
}

/// The armed entries, recovered from a poisoned lock: no code panics while holding it, and the
/// entries stay consistent even if one did.
fn armed_entries() -> MutexGuard<'static, Vec<ArmedEntry>> {
    ARMED.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
#[path = "tests/registry.rs"]
mod tests;
