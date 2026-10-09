//! The process-wide shutdown signal: the one flag the `api-server` binary raises when the process
//! is asked to stop, so work that would otherwise outlive a graceful shutdown ends with it.
//!
//! **Role:** [`ShutdownSignal`], a one-way flag that begins once and wakes every waiter, and
//! [`process_shutdown`], the instance the whole process shares.
//! **Position:** `api_configuration`. `crates/api/api_server/src/bin/api_server.rs` begins [`process_shutdown`] when
//! SIGINT or SIGTERM arrives, as `axum::serve` starts its graceful drain;
//! the API's `authorize_event_stream` middleware waits on it and
//! closes every open SSE stream, which is what lets the drain finish.
//! **Signals & state:** each signal owns one `tokio::sync::watch` channel holding `false` until
//! [`ShutdownSignal::begin`] stores `true`; the process-wide instance lives in a static for the
//! life of the process.
//! **Invariants:** a begun signal never returns to not begun, and beginning twice is the same as
//! beginning once; beginning wakes every waiter, however many exist; a waiter created after the
//! signal began resolves on its first poll; a signal dropped before it began leaves its waiters
//! pending, because it can no longer begin.

use std::future::Future;
use std::sync::LazyLock;

use tokio::sync::watch;

/// A one-way flag: not begun until [`ShutdownSignal::begin`] runs, begun for ever after.
pub struct ShutdownSignal {
    begun: watch::Sender<bool>,
}

impl ShutdownSignal {
    /// A signal that has not begun.
    pub fn new() -> Self {
        Self {
            begun: watch::channel(false).0,
        }
    }

    /// Begins the shutdown and wakes every waiter; beginning again changes nothing.
    pub fn begin(&self) {
        self.begun.send_replace(true);
    }

    /// `true` once [`ShutdownSignal::begin`] has run.
    pub fn has_begun(&self) -> bool {
        *self.begun.borrow()
    }

    /// A future that resolves once the shutdown begins, or on its first poll when it already
    /// has. It owns its own receiver, so it outlives the borrow of `self` and can be raced
    /// inside a long-lived stream.
    pub fn begun(&self) -> impl Future<Output = ()> + Send + 'static {
        let mut receiver = self.begun.subscribe();
        async move {
            let began = receiver.wait_for(|begun| *begun).await.is_ok();
            if !began {
                // The signal was dropped before it began: it can no longer begin.
                std::future::pending::<()>().await;
            }
        }
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

/// The shutdown signal of this process.
static PROCESS_SHUTDOWN: LazyLock<ShutdownSignal> = LazyLock::new(ShutdownSignal::new);

/// The shutdown signal the whole process shares: the `api-server` binary begins it on SIGINT or
/// SIGTERM, and every open event stream ends when it does.
pub fn process_shutdown() -> &'static ShutdownSignal {
    &PROCESS_SHUTDOWN
}

#[cfg(test)]
#[path = "tests/shutdown_signal.rs"]
mod tests;
