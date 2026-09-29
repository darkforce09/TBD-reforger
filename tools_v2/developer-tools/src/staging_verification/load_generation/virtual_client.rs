//! One virtual client: a pinned source address, a share of the accounts, and two lanes driven
//! side by side, one for its account switches and one for its member requests.
//!
//! - **Role:** builds the client's address-bound connection, its account-switch and member-slot
//!   schedules and its seeded streams, gives the ring of refresh tokens to the account-switch
//!   lane and the per-account request state to the member-request lane, drives both lanes to the
//!   end of the run, and merges what they recorded.
//! - **Position:** built and spawned by [`super::run`]; its lanes are
//!   [`super::account_switch_lane::AccountSwitchLane`] and
//!   [`super::member_request_lane::MemberRequestLane`], which both send through
//!   [`super::guarded_exchange::ClientConnection`].
//! - **Signals & state:** the lanes share one connection and one `watch` handover of the
//!   signed-in account; the client shares only the address guards and the [`RunContext`].
//! - **Invariants:**
//!   - A client has at most one member request and one refresh in flight, both from its own
//!     address and under that address's guard.
//!   - The member requests never wait on a refresh: the lanes run concurrently in the client's
//!     task, and only the handover passes between them.

use std::sync::Arc;

use anyhow::Result;
use tokio::time::Instant;

use super::account_rotation::ClientAccounts;
use super::account_switch_lane::AccountSwitchLane;
use super::guarded_exchange::ClientConnection;
use super::latency_recording::RequestRecord;
use super::member_request_lane::MemberRequestLane;
use super::pacing::{AccountSwitchSchedule, MemberSlotSchedule, RandomStream, SeededRandom};
use super::request_catalog::RequestCatalog;
use super::source_address_pool::SourceAddressPool;
use super::workload_plan::RunSettings;

/// What every client of one run shares.
pub(crate) struct RunContext {
    /// `http://host[:port]`, without a trailing slash.
    pub(crate) origin: String,
    pub(crate) start: Instant,
    /// `start + ramp + measured`: no event starts after it.
    pub(crate) end: Instant,
    pub(crate) settings: RunSettings,
    pub(crate) catalog: RequestCatalog,
    pub(crate) pool: SourceAddressPool,
}

/// What a client leaves behind when its run ends.
#[derive(Debug)]
pub(crate) struct ClientOutcome {
    pub(crate) address: usize,
    pub(crate) records: Vec<RequestRecord>,
    pub(crate) skipped_slots: u64,
    pub(crate) unsent_slots: u64,
    pub(crate) guard_delayed: u64,
    pub(crate) late_switches: u64,
}

/// One client's two lanes for the length of a run.
pub(crate) struct VirtualClient {
    address: usize,
    switches: AccountSwitchLane,
    requests: MemberRequestLane,
}

impl VirtualClient {
    pub(crate) fn new(
        index: u32,
        settings: &RunSettings,
        pool: &SourceAddressPool,
        seed: u64,
        accounts: ClientAccounts,
    ) -> Result<Self> {
        let address = pool.address_for_client(index);
        let connection =
            ClientConnection::new(index, address, pool.ip(address), settings.request_timeout)?;
        let schedule = AccountSwitchSchedule::new(settings, index);
        let mut pacing_random = SeededRandom::for_stream(seed, index, RandomStream::Pacing);
        let slots = MemberSlotSchedule::new(settings, index, &mut pacing_random);
        let (switches, signed_in) =
            AccountSwitchLane::new(connection.clone(), schedule, accounts.ring);
        let requests = MemberRequestLane {
            connection,
            slots,
            sign_in: schedule.sign_in(),
            pacing_random,
            mix_random: SeededRandom::for_stream(seed, index, RandomStream::Mix),
            accounts: accounts.members,
            signed_in,
        };
        Ok(Self {
            address,
            switches,
            requests,
        })
    }

    /// Drive both lanes until the window closes and their last exchanges end.
    pub(crate) async fn run(self, context: Arc<RunContext>) -> ClientOutcome {
        let (switched, requested) = tokio::join!(
            self.switches.run(Arc::clone(&context)),
            self.requests.run(context)
        );
        let mut records = requested.records;
        records.extend(switched.records);
        ClientOutcome {
            address: self.address,
            records,
            skipped_slots: requested.skipped_slots,
            unsent_slots: requested.unsent_slots,
            guard_delayed: requested.guard_delayed + switched.guard_delayed,
            late_switches: switched.late_switches,
        }
    }
}
