//! The member side of a virtual client: its paced slots, each sent as the account it holds.
//!
//! - **Role:** walks the client's paced slots from its sign-in instant to the end of the run's
//!   window, sends each slot's pick from the request mix as the account the account-switch lane
//!   has handed over, and records every exchange.
//! - **Position:** one of the two lanes [`crate::virtual_client::VirtualClient::run`] drives side
//!   by side; reads the handover of [`crate::account_switch_lane`] at every slot, resolves
//!   requests through the request catalog and sends through [`crate::guarded_exchange`].
//! - **Signals & state:** owns the slot schedule, the pacing and mix streams, every account's
//!   request state, the receiving half of the handover, and its records and slot counts.
//! - **Invariants:**
//!   - A slot is sent only when the client already holds an account at its scheduled instant; a
//!     slot that finds none is skipped, never sent late, so no member request waits for a token
//!     and a client joins the member load with the first slot after its sign-in has ended.
//!   - At most one member request is in flight: a slot that comes due during an exchange starts
//!     as soon as it ends, and its latency still counts from its scheduled instant.
//!   - No slot starts once the run's window has closed; the slots still waiting then count as
//!     unsent. A slot started before the close may still wait out its address's ceilings.

use std::sync::Arc;
use std::time::Duration;

use staging_load_plan::latency_recording::RequestRecord;
use staging_load_plan::pacing::{MemberSlotSchedule, SeededRandom};
use staging_load_plan::request_catalog;
use tokio::time::Instant;

use crate::account_rotation::MemberAccount;
use crate::account_switch_lane::{SignedInAccount, SignedInReceiver};
use crate::guarded_exchange::{BodyUse, ClientConnection, RequestHeaders};
use crate::virtual_client::RunContext;

/// What the member-request lane leaves behind when the run ends.
#[derive(Debug, Default)]
pub(crate) struct MemberRequestOutcome {
    pub(crate) records: Vec<RequestRecord>,
    pub(crate) skipped_slots: u64,
    pub(crate) unsent_slots: u64,
    pub(crate) guard_delayed: u64,
}

/// The member side of one client for the length of a run.
pub(crate) struct MemberRequestLane {
    pub(crate) connection: ClientConnection,
    pub(crate) slots: MemberSlotSchedule,
    /// The client's sign-in instant, from which its slots count.
    pub(crate) sign_in: Duration,
    pub(crate) pacing_random: SeededRandom,
    pub(crate) mix_random: SeededRandom,
    pub(crate) accounts: Vec<MemberAccount>,
    pub(crate) signed_in: SignedInReceiver,
}

impl MemberRequestLane {
    /// Send every slot from the sign-in on until the window closes.
    pub(crate) async fn run(mut self, context: Arc<RunContext>) -> MemberRequestOutcome {
        let mut outcome = MemberRequestOutcome::default();
        let window_end = context.settings.ramp + context.settings.measured;
        let mut slot = self
            .slots
            .first_slot_from(self.sign_in, &mut self.pacing_random);
        loop {
            if slot >= window_end || Instant::now() >= context.end {
                break;
            }
            tokio::time::sleep_until(context.start + slot).await;
            let held = self.signed_in.borrow().clone();
            match held {
                Some(account) => self.send_slot(&context, slot, account, &mut outcome).await,
                None => outcome.skipped_slots += 1,
            }
            slot = self.slots.next_slot(&mut self.pacing_random);
        }
        while slot < window_end {
            outcome.unsent_slots += 1;
            slot = self.slots.next_slot(&mut self.pacing_random);
        }
        outcome
    }

    /// Send one pick from the mix as `signed_in`.
    async fn send_slot(
        &mut self,
        context: &RunContext,
        scheduled: Duration,
        signed_in: SignedInAccount,
        outcome: &mut MemberRequestOutcome,
    ) {
        let template_index = context.catalog.pick(self.mix_random.next_u64());
        let template = context.catalog.template(template_index);
        let account = &self.accounts[signed_in.position];
        let step = &template.steps[account.step_cursor(template_index) % template.steps.len()];
        let request = request_catalog::resolve(step, &account.binding);
        let if_none_match = request
            .conditional
            .then(|| account.entity_tag(&request.path).map(str::to_owned))
            .flatten();
        let account_index = account.index;
        let (conditional, auth, path) = (request.conditional, request.auth, request.path.clone());
        let headers = RequestHeaders {
            bearer: Some(&signed_in.access_token),
            if_none_match: if_none_match.as_deref(),
        };
        let exchange = self
            .connection
            .exchange(context, scheduled, request, headers, BodyUse::Discard)
            .await;
        outcome.guard_delayed += u64::from(exchange.guard_delayed);
        if exchange.outcome.is_expected() {
            let account = &mut self.accounts[signed_in.position];
            account.advance_step(template_index);
            if let (true, Some(tag)) = (conditional, exchange.entity_tag) {
                account.remember_entity_tag(path, tag);
            }
        }
        outcome.records.push(RequestRecord {
            client: self.connection.client,
            address: self.connection.address,
            account: account_index,
            class: template.class,
            template: Some(template_index),
            scheduled,
            sent: exchange.sent,
            finished: exchange.finished,
            outcome: exchange.outcome,
            auth,
        });
    }
}
