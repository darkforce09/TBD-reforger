//! A session-keyed in-flight future shared by concurrent callers in the current generation.
//!
//! **Role:** guarantees that a value which must be produced at most once at a time — a rotated
//! refresh token — is produced once however many callers ask for it together.
//! **Position:** the cell type of the HTTP client's refresh-and-retry contract; the one cell per
//! browsing context is held by the session refresh in `foundation::auth`.
//! **Signals & state:** holds the shared future in a reference-counted cell. Not a Leptos signal.
//! **Invariants:** callers share only the current generation's future. Any completing waiter
//! clears that specific flight, so cancellation cannot leave a completed result cached and an
//! older completion cannot clear a newer flight. Reference counting allows clones to outlive
//! a thread-local borrow across an await.

#[cfg(any(target_arch = "wasm32", test))]
use futures::future::{FutureExt, LocalBoxFuture, Shared};
#[cfg(any(target_arch = "wasm32", test))]
use std::cell::RefCell;
#[cfg(any(target_arch = "wasm32", test))]
use std::future::Future;
#[cfg(any(target_arch = "wasm32", test))]
use std::rc::Rc;

/// The flight currently available for callers to join.
#[cfg(any(target_arch = "wasm32", test))]
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
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone)]
pub struct SingleFlight<T: Clone> {
    inflight: Rc<RefCell<Option<InFlight<T>>>>,
}

#[cfg(any(target_arch = "wasm32", test))]
impl<T: Clone + 'static> SingleFlight<T> {
    /// An empty cell, with nothing in flight.
    #[cfg(any(target_arch = "wasm32", test))]
    pub fn new() -> Self {
        Self {
            inflight: Rc::new(RefCell::new(None)),
        }
    }

    /// Share a flight using generation zero when session isolation is unnecessary.
    #[cfg(test)]
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
    #[cfg(any(target_arch = "wasm32", test))]
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

/// An empty cell, the same as [`SingleFlight::new()`].
#[cfg(any(target_arch = "wasm32", test))]
impl<T: Clone + 'static> Default for SingleFlight<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "tests/single_flight.rs"]
mod tests;
