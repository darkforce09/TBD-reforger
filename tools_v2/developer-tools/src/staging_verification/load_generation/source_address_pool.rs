//! The run's source addresses and the per-address ceilings every request passes.
//!
//! - **Role:** pins client `c` to address `c mod n`, confirms that every address is assigned to
//!   this machine, and reserves each request's send instant so that no address exceeds its
//!   ceilings; also measures the busiest window of a list of send instants for the report.
//! - **Position:** built by [`super::run`] from the plan's addresses and ceilings; both lanes of
//!   every virtual client reserve through it before each send; the report measures with
//!   [`busiest_window`].
//! - **Signals & state:** one mutex-guarded [`AddressGuard`] per address holding its most recent
//!   reservations; the lock is held only while a reservation is computed, never across an await.
//! - **Invariants:**
//!   - The reservations of one address never decrease, and for a ceiling of `N` requests per `W`
//!     the `k`-th and `(k+N)`-th are at least `W +` [`GUARD_MARGIN`] apart, so no window of length
//!     `W` holds more than `N` sends; an auth request passes both ceilings.
//!   - A reservation is never earlier than the instant it was asked for.
//!   - An auth request is reserved only at an instant its auth ceiling already allows at its place
//!     in the address's order; while that ceiling holds it back it reserves nothing, so the
//!     address's queue never stands still behind it and no member request waits on a refresh.

use std::collections::VecDeque;
use std::net::{IpAddr, TcpListener};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::time::Instant;

use super::workload_plan::{CeilingWindow, RunSettings};

/// Slack added to every ceiling window, so a send the scheduler wakes a little late still leaves
/// its window clear.
pub(crate) const GUARD_MARGIN: Duration = Duration::from_millis(50);

/// Check that every address is assigned to this machine by binding an ephemeral port on it.
///
/// # Errors
///
/// Names the first address the operating system refuses to bind.
pub fn verify_source_addresses(addresses: &[IpAddr]) -> Result<()> {
    for address in addresses {
        TcpListener::bind((*address, 0))
            .with_context(|| format!("source address {address} is not assigned to this machine"))?;
    }
    Ok(())
}

/// One sliding-window ceiling and the reservations it has granted most recently.
#[derive(Debug)]
struct SlidingCeiling {
    max_requests: usize,
    spacing: Duration,
    recent: VecDeque<Instant>,
}

impl SlidingCeiling {
    fn new(ceiling: CeilingWindow) -> Self {
        Self {
            max_requests: ceiling.max_requests,
            spacing: ceiling.window + GUARD_MARGIN,
            recent: VecDeque::with_capacity(ceiling.max_requests),
        }
    }

    /// The earliest instant at or after `at` this ceiling allows.
    fn earliest(&self, at: Instant) -> Instant {
        match self.recent.front() {
            Some(&oldest) if self.recent.len() == self.max_requests => {
                at.max(oldest + self.spacing)
            }
            _ => at,
        }
    }

    fn record(&mut self, at: Instant) {
        if self.recent.len() == self.max_requests {
            self.recent.pop_front();
        }
        self.recent.push_back(at);
    }
}

/// The ceilings of one source address.
#[derive(Debug)]
pub(crate) struct AddressGuard {
    all_requests: SlidingCeiling,
    auth_requests: SlidingCeiling,
    last_reserved: Option<Instant>,
}

impl AddressGuard {
    pub(crate) fn new(all_requests: CeilingWindow, auth_requests: CeilingWindow) -> Self {
        Self {
            all_requests: SlidingCeiling::new(all_requests),
            auth_requests: SlidingCeiling::new(auth_requests),
            last_reserved: None,
        }
    }

    /// The earliest instant at or after `requested` that follows every earlier reservation of
    /// the address and keeps its all-requests ceiling.
    fn queued(&self, requested: Instant) -> Instant {
        let at = self
            .last_reserved
            .map_or(requested, |last| requested.max(last));
        self.all_requests.earliest(at)
    }

    /// Reserve the earliest send instant at or after `requested` for a request outside the auth
    /// routes.
    pub(crate) fn reserve(&mut self, requested: Instant) -> Instant {
        let at = self.queued(requested);
        self.all_requests.record(at);
        self.last_reserved = Some(at);
        at
    }

    /// Reserve an auth request at its place in the address's order when the auth ceiling allows
    /// it there; otherwise reserve nothing and return the instant from which that ceiling allows
    /// it, to ask again then.
    pub(crate) fn reserve_auth(&mut self, requested: Instant) -> Result<Instant, Instant> {
        let at = self.queued(requested);
        let allowed = self.auth_requests.earliest(at);
        if allowed > at {
            return Err(allowed);
        }
        self.all_requests.record(at);
        self.auth_requests.record(at);
        self.last_reserved = Some(at);
        Ok(at)
    }
}

/// The run's source addresses, each with its guard.
#[derive(Debug)]
pub(crate) struct SourceAddressPool {
    addresses: Vec<(IpAddr, Mutex<AddressGuard>)>,
}

impl SourceAddressPool {
    pub(crate) fn new(addresses: &[IpAddr], settings: &RunSettings) -> Self {
        Self {
            addresses: addresses
                .iter()
                .map(|&address| {
                    let guard = AddressGuard::new(settings.all_requests, settings.auth_requests);
                    (address, Mutex::new(guard))
                })
                .collect(),
        }
    }

    /// The address client `client` sends from: `client mod n`.
    pub(crate) fn address_for_client(&self, client: u32) -> usize {
        client as usize % self.addresses.len()
    }

    pub(crate) fn ip(&self, index: usize) -> IpAddr {
        self.addresses[index].0
    }

    fn guard(&self, index: usize) -> MutexGuard<'_, AddressGuard> {
        self.addresses[index]
            .1
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Reserve a send instant on address `index`; see [`AddressGuard::reserve`].
    pub(crate) fn reserve_send(&self, index: usize, requested: Instant) -> Instant {
        self.guard(index).reserve(requested)
    }

    /// Reserve an auth send on address `index`; see [`AddressGuard::reserve_auth`].
    pub(crate) fn reserve_auth_send(
        &self,
        index: usize,
        requested: Instant,
    ) -> Result<Instant, Instant> {
        self.guard(index).reserve_auth(requested)
    }
}

/// The most instants of `sorted` (ascending) inside any half-open window of length `window`.
pub(crate) fn busiest_window(sorted: &[Duration], window: Duration) -> usize {
    let mut busiest = 0;
    let mut first = 0;
    for (last, &instant) in sorted.iter().enumerate() {
        while instant - sorted[first] >= window {
            first += 1;
        }
        busiest = busiest.max(last - first + 1);
    }
    busiest
}

#[cfg(test)]
#[path = "tests/source_address_pool_tests.rs"]
mod tests;
