//! One in-flight future per key, shared by every concurrent caller.
//!
//! **Role:** guarantees that a value which must be produced at most once at a time — a rotated
//! refresh token — is produced once however many callers ask for it together.
//! **Position:** used by the HTTP client's refresh path, one cell per browsing context.
//! **Signals & state:** holds the shared future in a reference-counted cell. Not a Leptos signal.
//! **Invariants:** the owner clears the cell once the future settles, so a later caller starts a
//! fresh attempt rather than replaying a spent result. Reference-counted rather than borrowed so
//! it can live in a thread-local and be cloned out across an await, which a thread-local borrow
//! cannot survive.

use futures::future::{FutureExt, LocalBoxFuture, Shared};
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

/// At most one in-flight future for `T` at a time.
///
/// Concurrent callers of [`SingleFlight::run`] await a clone of the same shared future; the caller
/// that started it clears the cell once it settles.
#[allow(dead_code)]
#[derive(Clone)]
pub struct SingleFlight<T: Clone> {
    inflight: Rc<RefCell<Option<Shared<LocalBoxFuture<'static, T>>>>>,
}

#[allow(dead_code)]
impl<T: Clone + 'static> SingleFlight<T> {
    /// An empty cell, with nothing in flight.
    pub fn new() -> Self {
        Self {
            inflight: Rc::new(RefCell::new(None)),
        }
    }

    /// Await the in-flight future, starting one with `make` when there is none.
    ///
    /// Every concurrent caller gets a clone of the same shared future, so `make` runs once. The
    /// caller that started it clears the cell once the future settles, which is what lets a later
    /// call start a fresh attempt rather than replay a spent result.
    pub async fn run<F, Fut>(&self, make: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T> + 'static,
    {
        let (shared, owner) = {
            let mut slot = self.inflight.borrow_mut();
            match slot.as_ref() {
                Some(s) => (s.clone(), false),
                None => {
                    let s = make().boxed_local().shared();
                    *slot = Some(s.clone());
                    (s, true)
                }
            }
        };
        let out = shared.await;
        if owner {
            *self.inflight.borrow_mut() = None;
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
