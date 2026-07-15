//! Auth — single-use refresh-token coordination. Ported from api/refresh.ts.
//!
//! Refresh tokens are single-use: the server rotates + revokes the old token on every
//! `/auth/refresh`. Several callers can want a refresh at once (the app-load bootstrap, the 401
//! retry interceptor, several requests 401-ing together). Without coordination they'd each present
//! the same token and all but the first would 401 — wrongly clearing the session. [`SingleFlight`]
//! guarantees the token is spent at most once at a time: concurrent callers await a clone of one
//! in-flight future, and the cell clears when it settles so the next call starts fresh. This is the
//! exact invariant of refresh.ts's module-level `inflight`, re-expressed for wasm's single-threaded
//! async. The gloo-net POST is wired on top (wasm) in the client slice; the coordination here is
//! pure and unit-tested natively (the "N concurrent → exactly one refresh" proof).

use futures::future::{FutureExt, LocalBoxFuture, Shared};
use std::cell::RefCell;
use std::future::Future;

/// At most one in-flight future for `T` at a time. Concurrent callers of [`run`](Self::run) await a
/// clone of the same shared future; the owner clears the cell once it settles.
// Wired to the gloo-net refresh + 401-retry client in the next auth slice; unit-tested now.
#[allow(dead_code)]
pub struct SingleFlight<T: Clone> {
    inflight: RefCell<Option<Shared<LocalBoxFuture<'static, T>>>>,
}

#[allow(dead_code)]
impl<T: Clone + 'static> SingleFlight<T> {
    pub fn new() -> Self {
        Self { inflight: RefCell::new(None) }
    }

    /// Run `make` under single-flight. The first caller builds the future and stores it; concurrent
    /// callers clone the in-flight handle instead of calling `make` again. Cleared on settle,
    /// mirroring `.finally(() => (inflight = null))` in refresh.ts.
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
impl<T: Clone + 'static> Default for SingleFlight<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use futures::future::join_all;
    use std::cell::Cell;
    use std::pin::Pin;
    use std::rc::Rc;
    use std::task::{Context, Poll};

    /// Pends exactly once so several tasks all register on the shared future before it resolves —
    /// the overlap a real 401 storm creates.
    struct YieldOnce(bool);
    impl Future for YieldOnce {
        type Output = ();
        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            if self.0 {
                Poll::Ready(())
            } else {
                self.0 = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    // The load-bearing auth proof: N concurrent callers spend the single-use token exactly once.
    #[test]
    fn concurrent_callers_spend_the_token_once() {
        let calls = Rc::new(Cell::new(0));
        let sf = SingleFlight::<i32>::new();
        let mk = || {
            let calls = calls.clone();
            move || {
                calls.set(calls.get() + 1); // one increment == one refresh POST
                async {
                    YieldOnce(false).await;
                    42
                }
            }
        };
        let out = block_on(join_all(vec![sf.run(mk()), sf.run(mk()), sf.run(mk()), sf.run(mk())]));
        assert_eq!(calls.get(), 1, "four concurrent callers must trigger exactly one refresh");
        assert_eq!(out, vec![42, 42, 42, 42], "all callers receive the same rotated result");
    }

    // After a refresh settles the cell clears, so a later (non-overlapping) call refreshes again.
    #[test]
    fn cell_clears_after_settle() {
        let calls = Rc::new(Cell::new(0));
        let sf = SingleFlight::<i32>::new();
        let mk = || {
            let calls = calls.clone();
            move || {
                calls.set(calls.get() + 1);
                async { 7 }
            }
        };
        block_on(sf.run(mk()));
        block_on(sf.run(mk()));
        assert_eq!(calls.get(), 2, "sequential non-overlapping calls each refresh");
    }
}
