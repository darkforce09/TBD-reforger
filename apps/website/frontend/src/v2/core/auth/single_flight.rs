//! A session-keyed in-flight future shared by concurrent callers in the current generation.
//!
//! **Role:** guarantees that a value which must be produced at most once at a time — a rotated
//! refresh token — is produced once however many callers ask for it together.
//! **Position:** used by the HTTP client's refresh path, one cell per browsing context.
//! **Signals & state:** holds the shared future in a reference-counted cell. Not a Leptos signal.
//! **Invariants:** callers share only the current generation's future. Any completing waiter
//! clears that specific flight, so cancellation cannot leave a completed result cached and an
//! older completion cannot clear a newer flight. Reference counting allows clones to outlive
//! a thread-local borrow across an await.

use futures::future::{FutureExt, LocalBoxFuture, Shared};
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

/// The flight currently available for callers to join.
#[derive(Clone)]
struct InFlight<T: Clone> {
    generation: u64,
    identity: Rc<()>,
    future: Shared<LocalBoxFuture<'static, T>>,
}

/// One joinable in-flight future for the current generation.
///
/// A generation change starts a separate flight. Existing waiters retain their own shared
/// future, and their completion cannot remove its replacement from the cell.
#[allow(dead_code)]
#[derive(Clone)]
pub struct SingleFlight<T: Clone> {
    inflight: Rc<RefCell<Option<InFlight<T>>>>,
}

#[allow(dead_code)]
impl<T: Clone + 'static> SingleFlight<T> {
    /// An empty cell, with nothing in flight.
    pub fn new() -> Self {
        Self {
            inflight: Rc::new(RefCell::new(None)),
        }
    }

    /// Share a flight using generation zero when session isolation is unnecessary.
    pub async fn run<F, Fut>(&self, make: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T> + 'static,
    {
        self.run_keyed(0, make).await
    }

    /// Share the current flight only when it belongs to `generation`.
    ///
    /// Every completing waiter clears its own flight if that flight still occupies the cell.
    /// This remains true when the initiating waiter is cancelled or another generation starts.
    pub async fn run_keyed<F, Fut>(&self, generation: u64, make: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T> + 'static,
    {
        let flight = {
            let mut slot = self.inflight.borrow_mut();
            match slot.as_ref() {
                Some(flight) if flight.generation == generation => flight.clone(),
                _ => {
                    let flight = InFlight {
                        generation,
                        identity: Rc::new(()),
                        future: make().boxed_local().shared(),
                    };
                    *slot = Some(flight.clone());
                    flight
                }
            }
        };
        let out = flight.future.await;
        let mut slot = self.inflight.borrow_mut();
        if slot
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(&current.identity, &flight.identity))
        {
            *slot = None;
        }
        out
    }
}

#[allow(dead_code)]
/// An empty cell, the same as [`SingleFlight::new()`].
impl<T: Clone + 'static> Default for SingleFlight<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "tests/single_flight.rs"]
mod tests;
