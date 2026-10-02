//! The cold-start restore of a stored session, which every request waits for.
//!
//! **Role:** tells the request paths when the store knows which session they are sent under.
//! **Position:** a field of the session store. `bootstrap` settles it once the locked read of the
//! persisted session has restored it or found none; the request paths await it before they
//! capture the session generation.
//! **Signals & state:** whether the restore has settled, and the requests parked until it does.
//! **Invariants:** restoring a stored session advances the session generation, and a request
//! captured under an earlier generation is discarded when its answer lands. A request issued
//! during the cold start therefore waits until the restore has settled and is sent under the
//! restored generation. Settling is one-way and idempotent: once the store knows its session,
//! later session changes are ordinary generation changes.

use futures::channel::oneshot;
use leptos::prelude::*;

/// Whether the cold-start session restore has settled, and the requests waiting for it.
#[derive(Clone, Copy)]
pub struct SessionRestore {
    settled: RwSignal<bool>,
    waiters: StoredValue<Vec<oneshot::Sender<()>>, LocalStorage>,
}

impl SessionRestore {
    /// A restore that has already settled, or one that still has to.
    pub fn new(settled: bool) -> Self {
        Self {
            settled: RwSignal::new(settled),
            waiters: StoredValue::new_local(Vec::new()),
        }
    }

    /// Whether the store knows which session requests are sent under.
    pub fn is_settled(self) -> bool {
        self.settled.get_untracked()
    }

    /// Wait until the restore has settled; returns at once when it already has.
    ///
    /// A store whose owner is disposed releases its waiters rather than parking them forever.
    pub async fn wait(self) {
        if self.is_settled() {
            return;
        }
        let (sender, receiver) = oneshot::channel();
        if self
            .waiters
            .try_update_value(|waiters| waiters.push(sender))
            .is_none()
        {
            return;
        }
        let _ = receiver.await;
    }

    /// Mark the restore settled and release every waiting request.
    pub fn settle(self) {
        if self.is_settled() {
            return;
        }
        self.settled.set(true);
        let waiters = self
            .waiters
            .try_update_value(std::mem::take)
            .unwrap_or_default();
        for waiter in waiters {
            let _ = waiter.send(());
        }
    }
}

#[cfg(test)]
#[path = "tests/session_restore.rs"]
mod tests;
