//! The handle a test keeps for a paused failpoint: it learns when a request arrived there and
//! lets the request go on.
//!
//! **Role:** [`PauseHandle`], a single-use rendezvous between the one request an armed `Pause`
//! holds and the test that armed it.
//! **Position:** `api_failpoints`, compiled only with the `failpoints` feature; the test creates
//! it and arms it inside `FailAction::Pause`, the registry holds the first arrival on it, and the
//! `ArmGuard` releases it when dropped.
//! **Signals & state:** two `tokio::sync::watch` flags shared by every clone: `reached`, set when
//! the held arrival gets to the point, and `released`, set by [`PauseHandle::release`].
//! **Invariants:** both flags only ever go from `false` to `true`; the held arrival waits until
//! `released` is set, at once when it already is; [`PauseHandle::reached`] resolves once `reached`
//! is set and panics after [`PAUSE_REACH_BOUND`] (or the bound given to
//! [`PauseHandle::reached_within`]), so a point no request reaches fails the case instead of
//! hanging the binary.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;

/// How long [`PauseHandle::reached`] waits for a request to arrive before it fails the case.
pub const PAUSE_REACH_BOUND: Duration = Duration::from_secs(30);

/// A single-use rendezvous with the request a `Pause` failpoint holds; clones share it.
#[derive(Clone, Debug)]
pub struct PauseHandle {
    flags: Arc<PauseFlags>,
}

/// The two one-way flags every clone of a [`PauseHandle`] shares.
#[derive(Debug)]
struct PauseFlags {
    reached: watch::Sender<bool>,
    released: watch::Sender<bool>,
}

impl PauseHandle {
    /// A pause no request has reached and the test has not released.
    pub fn new() -> Self {
        Self {
            flags: Arc::new(PauseFlags {
                reached: watch::channel(false).0,
                released: watch::channel(false).0,
            }),
        }
    }

    /// Resolves once the held request has arrived at the point.
    ///
    /// # Panics
    ///
    /// When no request arrives within [`PAUSE_REACH_BOUND`].
    pub async fn reached(&self) {
        self.reached_within(PAUSE_REACH_BOUND).await;
    }

    /// Resolves once the held request has arrived at the point, waiting at most `bound`.
    ///
    /// # Panics
    ///
    /// When no request arrives within `bound`.
    pub async fn reached_within(&self, bound: Duration) {
        let mut reached = self.flags.reached.subscribe();
        let arrived = tokio::time::timeout(bound, reached.wait_for(|flag| *flag))
            .await
            .is_ok_and(|outcome| outcome.is_ok());
        assert!(
            arrived,
            "no request reached the paused failpoint within {bound:?}"
        );
    }

    /// Whether the held request has arrived at the point, without waiting.
    pub fn is_reached(&self) -> bool {
        *self.flags.reached.borrow()
    }

    /// Lets the held request go on; releasing again changes nothing.
    pub fn release(&self) {
        self.flags.released.send_replace(true);
    }

    /// Whether the pause has been released.
    pub fn is_released(&self) -> bool {
        *self.flags.released.borrow()
    }

    /// Marks the point reached and waits until the pause is released: the registry runs this for
    /// the arrival it holds.
    pub(super) async fn hold(&self) {
        self.flags.reached.send_replace(true);
        let mut released = self.flags.released.subscribe();
        // The sender lives in `self.flags`, which this future borrows, so the channel cannot
        // close while it waits and the result is always `Ok`.
        let _ = released.wait_for(|flag| *flag).await;
    }
}

impl Default for PauseHandle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "tests/pause_handle.rs"]
mod tests;
