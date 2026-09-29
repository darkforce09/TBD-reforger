//! Seeded open-loop pacing: each client's sign-in, account switches and paced member slots.
//!
//! - **Role:** turns the run settings and a client index into the offsets, from the run start, at
//!   which the client signs in, prefetches and switches accounts, and sends its member slots;
//!   supplies the seeded random streams behind every draw of a run; and counts the member accounts
//!   a run's window reaches.
//! - **Position:** each virtual client owns one [`AccountSwitchSchedule`], one
//!   [`MemberSlotSchedule`] and two [`SeededRandom`] streams (pacing and mix), built from the
//!   workload's seed; the local rehearsal scales its thresholds with
//!   [`reachable_member_accounts`].
//! - **Signals & state:** a slot schedule counts the slots it has issued; a stream holds its
//!   generator state. Both belong to one client.
//! - **Invariants:**
//!   - Client `c` signs in at `ramp · c / clients`, so the clients of one address sign in
//!     `ramp · addresses / clients` apart: the plan check holds that at or under 80 % of the
//!     address's auth ceiling.
//!   - Switch `m ≥ 1` is at `sign-in + m·hold`; its prefetch starts half a hold earlier, inside
//!     the hold of the account it replaces.
//!   - The period is `P = clients / requests_per_second`. Slot `n` is at `phase + n·P + j` from
//!     the run start, never below zero, with the phase drawn once inside the client's own stratum
//!     `[c·P / clients, (c + 1)·P / clients)` and `j` drawn per slot in `[−jitter·P, +jitter·P)`.
//!     The strata spread every address's clients evenly across the period, so the paced load
//!     alone never bunches into an address's all-requests ceiling; the jitter does not
//!     accumulate, and with the jitter below half a period a client's slots never reorder.
//!   - Every draw comes from a SplitMix64 stream keyed by the seed, the client and the stream's
//!     purpose, so a run's schedule and picks depend on the plan alone.

use std::time::Duration;

use anyhow::Result;

use super::workload_plan::{RunSettings, WorkloadPlan};

/// SplitMix64's increment, the golden-ratio constant.
const GOLDEN_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

/// What a random stream is drawn for; each purpose gets a stream of its own per client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RandomStream {
    /// The phase and the per-slot jitter.
    Pacing = 1,
    /// The template picks.
    Mix = 2,
}

/// A SplitMix64 generator: small, fast, and well mixed for simulation draws.
#[derive(Debug, Clone)]
pub(crate) struct SeededRandom {
    state: u64,
}

/// SplitMix64's output function, a bijection on 64-bit words.
fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl SeededRandom {
    /// The stream of `client` for `stream`, under the run's `seed`.
    pub(crate) fn for_stream(seed: u64, client: u32, stream: RandomStream) -> Self {
        let key = (u64::from(client) << 8) | stream as u64;
        Self {
            state: mix64(seed ^ mix64(key.wrapping_add(GOLDEN_GAMMA))),
        }
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(GOLDEN_GAMMA);
        mix64(self.state)
    }

    /// A draw in `[0, 1)` with 53 bits of precision.
    pub(crate) fn next_unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// The offset from the run start at which `client` signs in: `ramp · client / clients`.
fn sign_in_offset(settings: &RunSettings, client: u32) -> Duration {
    settings
        .ramp
        .mul_f64(f64::from(client) / f64::from(settings.clients))
}

/// When one client signs in, prefetches its next accounts and switches to them.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AccountSwitchSchedule {
    sign_in: Duration,
    hold: Duration,
}

impl AccountSwitchSchedule {
    pub(crate) fn new(settings: &RunSettings, client: u32) -> Self {
        Self {
            sign_in: sign_in_offset(settings, client),
            hold: settings.hold,
        }
    }

    /// The client's first refresh, which signs it into its first account.
    pub(crate) fn sign_in(&self) -> Duration {
        self.sign_in
    }

    /// Switch `number` (from 1): the instant the client's next account takes over.
    pub(crate) fn switch(&self, number: u32) -> Duration {
        self.sign_in + self.hold * number
    }

    /// When the prefetch of switch `number` starts: half a hold before the switch.
    pub(crate) fn prefetch(&self, number: u32) -> Duration {
        self.switch(number).saturating_sub(self.hold / 2)
    }
}

/// When one client's paced member slots fall, as offsets from the run start.
#[derive(Debug, Clone)]
pub(crate) struct MemberSlotSchedule {
    phase: Duration,
    period: Duration,
    jitter_fraction: f64,
    slots_issued: u32,
}

impl MemberSlotSchedule {
    /// The slots of `client`, drawing its phase inside its stratum from `random`.
    pub(crate) fn new(settings: &RunSettings, client: u32, random: &mut SeededRandom) -> Self {
        let stratum = (f64::from(client) + random.next_unit()) / f64::from(settings.clients);
        Self {
            phase: settings.period.mul_f64(stratum),
            period: settings.period,
            jitter_fraction: settings.jitter_fraction,
            slots_issued: 0,
        }
    }

    /// The next paced slot, drawing its jitter from `random`.
    pub(crate) fn next_slot(&mut self, random: &mut SeededRandom) -> Duration {
        let nominal = self.phase + self.period * self.slots_issued;
        self.slots_issued += 1;
        let jitter =
            self.period.as_secs_f64() * self.jitter_fraction * (2.0 * random.next_unit() - 1.0);
        Duration::from_secs_f64((nominal.as_secs_f64() + jitter).max(0.0))
    }

    /// The first paced slot at or after `instant`; the slots before it are drawn and passed.
    pub(crate) fn first_slot_from(
        &mut self,
        instant: Duration,
        random: &mut SeededRandom,
    ) -> Duration {
        loop {
            let slot = self.next_slot(random);
            if slot >= instant {
                return slot;
            }
        }
    }
}

/// The member accounts a run of `workload` reaches when every refresh succeeds: per client, one
/// account for its sign-in and one for each switch that still leaves a whole period of the run's
/// window, at most the accounts of its ring.
///
/// # Errors
///
/// The workload's numbers are refused; see [`WorkloadPlan::validate`].
pub fn reachable_member_accounts(workload: &WorkloadPlan) -> Result<u64> {
    let settings = workload.checked_settings()?;
    let window_end = settings.ramp + settings.measured;
    let ring = u64::from(workload.accounts_per_client);
    let hold = settings.hold.as_nanos();
    Ok((0..settings.clients)
        .map(|client| {
            let first_answer = sign_in_offset(&settings, client) + settings.period;
            let reached = window_end
                .checked_sub(first_answer)
                .map_or(0, |room| 1 + room.as_nanos() / hold);
            u64::try_from(reached).unwrap_or(u64::MAX).min(ring)
        })
        .sum())
}

#[cfg(test)]
#[path = "tests/pacing_tests.rs"]
mod tests;
