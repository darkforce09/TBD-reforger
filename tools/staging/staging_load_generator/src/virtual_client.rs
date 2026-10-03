//! One virtual client: a pinned source address, a share of the accounts, and two lanes driven
//! side by side, one for its account switches and one for its member requests.
//!
//! - **Role:** builds the client's address-bound connection, its account-switch and member-slot
//!   schedules and its seeded streams, gives the ring of refresh tokens to the account-switch
//!   lane and the per-account request state to the member-request lane, drives both lanes to the
//!   end of the run, and merges what they recorded.
//! - **Position:** built and spawned by [`crate::run`]; its lanes are
//!   [`crate::account_switch_lane::AccountSwitchLane`] and
//!   [`crate::member_request_lane::MemberRequestLane`], which both send through
//!   [`crate::guarded_exchange::ClientConnection`].
//! - **Signals & state:** the lanes share one connection and one `watch` handover of the
//!   signed-in account; the client shares only the address guards and the [`RunContext`].
//! - **Invariants:**
//!   - A client has at most one member request and one refresh in flight, both from its own
//!     address and under that address's guard.
//!   - The member requests never wait on a refresh: the lanes run concurrently in the client's
//!     task, and only the handover passes between them.

use std::sync::Arc;

use staging_load_plan::client_outcome::ClientOutcome;
use staging_load_plan::pacing::{
    AccountSwitchSchedule, MemberSlotSchedule, RandomStream, SeededRandom,
};
use staging_load_plan::request_catalog::RequestCatalog;
use staging_load_plan::run_settings::RunSettings;
use tokio::time::Instant;

use crate::account_rotation::ClientAccounts;
use crate::account_switch_lane::AccountSwitchLane;
use crate::error::Result;
use crate::guarded_exchange::ClientConnection;
use crate::member_request_lane::MemberRequestLane;
use crate::source_address_pool::SourceAddressPool;

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
