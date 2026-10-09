//! The source addresses of a load run: their ownership check and the window measure of their sends.
//!
//! - **Role:** confirms that every source address is assigned to this machine, and measures the
//!   busiest window of a list of send instants for the report.
//! - **Position:** the load generator checks the plan's addresses before it builds a client and
//!   spaces every address's sends by [`GUARD_MARGIN`]; the report measures each address with
//!   [`busiest_window`]; the xtask load procedure checks the addresses before it asks to run.
//! - **Signals & state:** none; a bind of an ephemeral port that is released at once.
//! - **Invariants:** an address passes only when the operating system binds a port on it; the
//!   busiest window counts a half-open window, so two sends exactly one window apart never share
//!   it.

use std::net::{IpAddr, TcpListener};
use std::time::Duration;

use crate::error::{Error, Result};

/// Slack added to every ceiling window, so a send the scheduler wakes a little late still leaves
/// its window clear.
pub const GUARD_MARGIN: Duration = Duration::from_millis(50);

/// Check that every address is assigned to this machine by binding an ephemeral port on it.
///
/// # Errors
///
/// Names the first address the operating system refuses to bind.
pub fn verify_source_addresses(addresses: &[IpAddr]) -> Result<()> {
    for address in addresses {
        TcpListener::bind((*address, 0)).map_err(|error| Error::SourceAddressNotAssigned {
            address: *address,
            error,
        })?;
    }
    Ok(())
}

/// The most instants of `sorted` (ascending) inside any half-open window of length `window`.
pub fn busiest_window(sorted: &[Duration], window: Duration) -> usize {
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
