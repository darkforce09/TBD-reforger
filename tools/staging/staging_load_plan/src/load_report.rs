//! The load run's report: the figures the load cases of the receipt are judged on.
//!
//! - **Role:** folds every client's records into a [`LoadReport`]: rates, per-class latency, the
//!   concurrency census, member accounts, unexpected errors, refreshes, per-address traffic
//!   against the ceilings, and per-template statuses.
//! - **Position:** assembled by the load generator's `run` once every client has returned; the
//!   xtask load procedure serialises it into its observation journal and maps it onto the load
//!   cases.
//! - **Signals & state:** none; pure functions over the records.
//! - **Invariants:**
//!   - `completed_requests`, the class summaries and the census count only answers that finished
//!     inside the measured window, `[ramp, ramp + measured)` from the run start; unexpected
//!     errors, refreshes, addresses and templates cover the whole run, ramp included.
//!   - The busiest-window figures come from the instants requests actually left, so they measure
//!     the ceilings instead of restating the guard's reservations.
//!   - The report names addresses, template ids and counts: never a token, a header or a body.

use std::collections::BTreeMap;
use std::ops::Range;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::client_outcome::ClientOutcome;
use crate::concurrency_census::{census, member_accounts};
use crate::identifiers::TemplateId;
use crate::latency_recording::{RequestOutcome, RequestRecord, summarise};
use crate::run_settings::RunSettings;
use crate::source_addresses::busiest_window;
use crate::workload_plan::{LoadRunPlan, PerAddressCeilings, RequestClass};

/// Everything one load run measured.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadReport {
    /// The workload's seed.
    pub seed: u64,
    /// Virtual clients.
    pub clients: u32,
    /// Accounts in the account file.
    pub accounts: u64,
    /// The workload's paced rate over all clients, requests per second.
    pub target_requests_per_second: f64,
    /// Each client's period between paced slots, milliseconds.
    pub period_milliseconds: f64,
    /// Seconds over which the clients joined.
    pub ramp_seconds: f64,
    /// Seconds of the measured window.
    pub measured_seconds: f64,
    /// Requests that left over the whole run, refreshes included.
    pub requests_sent: u64,
    /// Answers of any status that finished inside the measured window.
    pub completed_requests: u64,
    /// `completed_requests` per measured second.
    pub achieved_requests_per_second: f64,
    /// Paced slots from a client's sign-in on that it passed because it held no usable account
    /// at their scheduled instant, before its first token included.
    pub skipped_slots: u64,
    /// Paced slots due inside the run that had not left when the window closed.
    pub unsent_slots: u64,
    /// Requests whose send a per-address ceiling held back.
    pub guard_delayed_requests: u64,
    /// Account switches whose prefetched refresh had not finished at the switch instant; the
    /// client kept sending as its current account until the refresh ended.
    pub late_switches: u64,
    /// Unexpected outcomes over the whole run.
    pub unexpected_errors: UnexpectedErrors,
    /// JSON reads that finished inside the measured window.
    pub json_reads: ClassSummary,
    /// JSON writes that finished inside the measured window.
    pub json_writes: ClassSummary,
    /// Refreshes that finished inside the measured window.
    pub sessions: ClassSummary,
    /// Width of the census windows, seconds.
    pub census_window_seconds: f64,
    /// Distinct clients with an expected answer in each whole census window, in order.
    pub census_windows: Vec<u32>,
    /// The smallest entry of `census_windows`; 0 when no whole window fits.
    pub minimum_concurrent_clients: u32,
    /// Accounts that refreshed successfully and received an expected JSON answer.
    pub member_accounts: u64,
    /// Every refresh of the run.
    pub refreshes: RefreshSummary,
    /// The ceilings each source address was held under.
    pub per_address_ceilings: PerAddressCeilings,
    /// Traffic per source address, in plan order.
    pub addresses: Vec<AddressSummary>,
    /// Outcomes per template of the mix, in mix order.
    pub templates: Vec<TemplateSummary>,
}

/// One class's answers inside the measured window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassSummary {
    /// Answers of any status.
    pub completed: u64,
    /// Answers with an expected status: the latency sample.
    pub expected: u64,
    /// Median latency of the sample by nearest rank, milliseconds.
    pub p50_milliseconds: Option<f64>,
    /// 95th-percentile latency of the sample by nearest rank, milliseconds.
    pub p95_milliseconds: Option<f64>,
    /// Largest latency of the sample, milliseconds.
    pub max_milliseconds: Option<f64>,
}

/// Unexpected outcomes by kind.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnexpectedErrors {
    /// Every unexpected outcome.
    pub total: u64,
    /// Responses with a status outside their step's list.
    pub unexpected_statuses: u64,
    /// Requests without a complete response.
    pub transport_errors: u64,
    /// Requests whose body had not ended when the timeout passed.
    pub timeouts: u64,
    /// Refreshes answered with an expected status but no complete token pair.
    pub undecodable_session_answers: u64,
}

/// The refreshes of the run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefreshSummary {
    /// Refreshes that left.
    pub attempted: u64,
    /// Refreshes that handed over a new token pair.
    pub succeeded: u64,
    /// Every status a refresh was answered with, and how often.
    pub statuses: BTreeMap<u16, u64>,
}

/// One source address's traffic over the whole run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddressSummary {
    /// The address.
    pub address: String,
    /// Clients sending from it.
    pub clients: u64,
    /// Requests that left from it.
    pub requests: u64,
    /// Of those, the requests under the authentication routes.
    pub auth_requests: u64,
    /// The most requests that left inside one all-requests ceiling window.
    pub busiest_window_requests: u64,
    /// The most auth requests that left inside one auth ceiling window.
    pub busiest_window_auth_requests: u64,
}

/// One template's outcomes over the whole run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateSummary {
    /// The template's id.
    pub id: TemplateId,
    /// The template's class.
    pub class: RequestClass,
    /// Every status its requests were answered with, and how often.
    pub statuses: BTreeMap<u16, u64>,
    /// Its unexpected outcomes.
    pub unexpected: u64,
    /// Its requests without a complete response.
    pub transport_errors: u64,
    /// Its requests that timed out.
    pub timeouts: u64,
}

/// Fold the clients' records into the report.
pub fn assemble(
    plan: &LoadRunPlan,
    settings: &RunSettings,
    outcomes: &[ClientOutcome],
) -> LoadReport {
    let records: Vec<RequestRecord> = outcomes
        .iter()
        .flat_map(|outcome| outcome.records.iter().copied())
        .collect();
    let measured = settings.ramp..settings.ramp + settings.measured;
    let completed_requests = count(&records, |record| {
        record.outcome.status().is_some() && measured.contains(&record.finished)
    });
    let census = census(&records, &measured, settings.census_window);
    LoadReport {
        seed: plan.workload.seed,
        clients: settings.clients,
        accounts: settings.accounts as u64,
        target_requests_per_second: plan.workload.requests_per_second,
        period_milliseconds: milliseconds(settings.period),
        ramp_seconds: settings.ramp.as_secs_f64(),
        measured_seconds: settings.measured.as_secs_f64(),
        requests_sent: records.len() as u64,
        completed_requests,
        achieved_requests_per_second: completed_requests as f64 / settings.measured.as_secs_f64(),
        skipped_slots: outcomes.iter().map(|outcome| outcome.skipped_slots).sum(),
        unsent_slots: outcomes.iter().map(|outcome| outcome.unsent_slots).sum(),
        guard_delayed_requests: outcomes.iter().map(|outcome| outcome.guard_delayed).sum(),
        late_switches: outcomes.iter().map(|outcome| outcome.late_switches).sum(),
        unexpected_errors: unexpected_errors(&records),
        json_reads: class_summary(&records, RequestClass::JsonRead, &measured),
        json_writes: class_summary(&records, RequestClass::JsonWrite, &measured),
        sessions: class_summary(&records, RequestClass::Session, &measured),
        census_window_seconds: settings.census_window.as_secs_f64(),
        census_windows: census.windows,
        minimum_concurrent_clients: census.minimum_concurrent_clients,
        member_accounts: member_accounts(&records),
        refreshes: refresh_summary(&records),
        per_address_ceilings: plan.workload.per_address_ceilings.clone(),
        addresses: address_summaries(plan, settings, outcomes, &records),
        templates: template_summaries(plan, &records),
    }
}

fn count(records: &[RequestRecord], predicate: impl Fn(&RequestRecord) -> bool) -> u64 {
    records.iter().filter(|record| predicate(record)).count() as u64
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn class_summary(
    records: &[RequestRecord],
    class: RequestClass,
    measured: &Range<Duration>,
) -> ClassSummary {
    let latency = summarise(records, class, measured);
    ClassSummary {
        completed: count(records, |record| {
            record.class == class
                && record.outcome.status().is_some()
                && measured.contains(&record.finished)
        }),
        expected: latency.samples as u64,
        p50_milliseconds: latency.p50.map(milliseconds),
        p95_milliseconds: latency.p95.map(milliseconds),
        max_milliseconds: latency.max.map(milliseconds),
    }
}

fn unexpected_errors(records: &[RequestRecord]) -> UnexpectedErrors {
    let mut tally = UnexpectedErrors::default();
    for record in records {
        match record.outcome {
            RequestOutcome::Expected { .. } => continue,
            RequestOutcome::UnexpectedStatus { .. } => tally.unexpected_statuses += 1,
            RequestOutcome::TransportError => tally.transport_errors += 1,
            RequestOutcome::Timeout => tally.timeouts += 1,
            RequestOutcome::UndecodableSessionAnswer { .. } => {
                tally.undecodable_session_answers += 1;
            }
        }
        tally.total += 1;
    }
    tally
}

fn refresh_summary(records: &[RequestRecord]) -> RefreshSummary {
    let mut summary = RefreshSummary::default();
    for record in records
        .iter()
        .filter(|record| record.class == RequestClass::Session)
    {
        summary.attempted += 1;
        if record.outcome.is_expected() {
            summary.succeeded += 1;
        }
        if let Some(status) = record.outcome.status() {
            *summary.statuses.entry(status).or_default() += 1;
        }
    }
    summary
}

fn address_summaries(
    plan: &LoadRunPlan,
    settings: &RunSettings,
    outcomes: &[ClientOutcome],
    records: &[RequestRecord],
) -> Vec<AddressSummary> {
    let sends = |index: usize, auth_only: bool| {
        let mut instants: Vec<Duration> = records
            .iter()
            .filter(|record| record.address == index && (record.auth || !auth_only))
            .map(|record| record.sent)
            .collect();
        instants.sort_unstable();
        instants
    };
    plan.source_addresses
        .iter()
        .enumerate()
        .map(|(index, address)| {
            let all = sends(index, false);
            let auth = sends(index, true);
            AddressSummary {
                address: address.to_string(),
                clients: outcomes
                    .iter()
                    .filter(|outcome| outcome.address == index)
                    .count() as u64,
                requests: all.len() as u64,
                auth_requests: auth.len() as u64,
                busiest_window_requests: busiest_window(&all, settings.all_requests.window) as u64,
                busiest_window_auth_requests: busiest_window(&auth, settings.auth_requests.window)
                    as u64,
            }
        })
        .collect()
}

fn template_summaries(plan: &LoadRunPlan, records: &[RequestRecord]) -> Vec<TemplateSummary> {
    let mut summaries: Vec<TemplateSummary> = plan
        .workload
        .request_mix
        .iter()
        .map(|template| TemplateSummary {
            id: template.id.clone(),
            class: template.class,
            statuses: BTreeMap::new(),
            unexpected: 0,
            transport_errors: 0,
            timeouts: 0,
        })
        .collect();
    for record in records {
        let Some(summary) = record.template.and_then(|index| summaries.get_mut(index)) else {
            continue;
        };
        if let Some(status) = record.outcome.status() {
            *summary.statuses.entry(status).or_default() += 1;
        }
        match record.outcome {
            RequestOutcome::TransportError => summary.transport_errors += 1,
            RequestOutcome::Timeout => summary.timeouts += 1,
            _ => {}
        }
        if !record.outcome.is_expected() {
            summary.unexpected += 1;
        }
    }
    summaries
}
