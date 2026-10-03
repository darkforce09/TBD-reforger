//! The member-load cases judged on the load engine's report.
//!
//! **Role:** decodes the [`LoadReport`] the run measured and judges `sustained_rate`,
//! `concurrency`, `member_accounts`, `zero_unexpected_errors`, `p95_json_reads`,
//! `p95_json_writes` and the report's half of `refresh_paced` against [`LoadThresholds`].
//!
//! **Position:** the judges of the plan's `member_load` effects, whose probes read the
//! `load_report` measurement; the local rehearsal judges its shortened run with them too.
//!
//! **Signals & state:** none; pure judges.
//!
//! **Invariants:** the acceptance thresholds are the ones `operational.rs` applies to the
//! receipt's observations (1,800 measured seconds, 20 completed requests a second, 100
//! concurrent clients, 1,000 member accounts, no unexpected error, p95 reads within 500 ms and
//! writes within 1,000 ms); a report that does not decode contradicts every case it decides.

use staging_load_plan::LoadReport;
use staging_load_plan::load_report::ClassSummary;

use crate::procedure_runner::step::{ProbeVerdict, StepContext};

/// The measurement holding the load engine's report.
pub(crate) const REPORT_MEASUREMENT: &str = "load_report";

/// The figures a load run must reach.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LoadThresholds {
    pub measured_seconds: f64,
    pub completed_requests_per_second: f64,
    pub concurrent_clients: u32,
    pub member_accounts: u64,
    pub p95_json_read_ms: f64,
    pub p95_json_write_ms: f64,
}

impl LoadThresholds {
    /// The register's acceptance of the `staging_load` receipt.
    pub(crate) const ACCEPTANCE: Self = Self {
        measured_seconds: 1_800.0,
        completed_requests_per_second: 20.0,
        concurrent_clients: 100,
        member_accounts: 1_000,
        p95_json_read_ms: 500.0,
        p95_json_write_ms: 1_000.0,
    };

    /// The acceptance over a shortened window of `measured_seconds` and `accounts` accounts, as
    /// the local rehearsal runs it.
    pub(crate) fn rehearsal(measured_seconds: f64, accounts: u64) -> Self {
        Self {
            measured_seconds,
            member_accounts: accounts.min(Self::ACCEPTANCE.member_accounts),
            ..Self::ACCEPTANCE
        }
    }
}

/// Decodes the report measurement.
pub(crate) fn report_of(text: &str) -> Result<LoadReport, ProbeVerdict> {
    serde_json::from_str(text).map_err(|error| {
        ProbeVerdict::Contradicted(format!("the load report is unreadable: {error}"))
    })
}

/// A judge of one member-load case.
pub(crate) type ReportJudge = fn(&LoadReport, &LoadThresholds) -> Result<String, String>;

/// The judge closure of `case` for a probe of [`REPORT_MEASUREMENT`].
pub(crate) fn report_judge(
    judge: ReportJudge,
    thresholds: LoadThresholds,
) -> impl Fn(&str, &StepContext<'_>) -> ProbeVerdict + 'static {
    move |text: &str, _: &StepContext<'_>| match report_of(text) {
        Err(verdict) => verdict,
        Ok(report) => match judge(&report, &thresholds) {
            Ok(summary) => ProbeVerdict::Satisfied(ProbeVerdict::satisfied(summary)),
            Err(why) => ProbeVerdict::Contradicted(why),
        },
    }
}

/// `sustained_rate`: the measured window lasted and completed enough requests.
pub(crate) fn sustained_rate(
    report: &LoadReport,
    limits: &LoadThresholds,
) -> Result<String, String> {
    let required = (report.measured_seconds * limits.completed_requests_per_second).ceil() as u64;
    let summary = format!(
        "{} requests completed in {} measured s ({:.2}/s; paced for {}/s; {} slots unsent)",
        report.completed_requests,
        report.measured_seconds,
        report.achieved_requests_per_second,
        report.target_requests_per_second,
        report.unsent_slots
    );
    if report.measured_seconds >= limits.measured_seconds && report.completed_requests >= required {
        Ok(summary)
    } else {
        Err(format!(
            "{summary}; needs {} measured s and {required} completed requests",
            limits.measured_seconds
        ))
    }
}

/// `concurrency`: every whole census window saw enough distinct clients.
pub(crate) fn concurrency(report: &LoadReport, limits: &LoadThresholds) -> Result<String, String> {
    let summary = format!(
        "at least {} distinct clients in each of {} census windows of {} s",
        report.minimum_concurrent_clients,
        report.census_windows.len(),
        report.census_window_seconds
    );
    if !report.census_windows.is_empty()
        && report.minimum_concurrent_clients >= limits.concurrent_clients
    {
        Ok(summary)
    } else {
        Err(format!("{summary}; needs {}", limits.concurrent_clients))
    }
}

/// `member_accounts`: enough accounts refreshed and received an expected JSON answer.
pub(crate) fn member_accounts(
    report: &LoadReport,
    limits: &LoadThresholds,
) -> Result<String, String> {
    let summary = format!(
        "{} member accounts of {} ({} of {} refreshes succeeded)",
        report.member_accounts,
        report.accounts,
        report.refreshes.succeeded,
        report.refreshes.attempted
    );
    if report.member_accounts >= limits.member_accounts {
        Ok(summary)
    } else {
        Err(format!("{summary}; needs {}", limits.member_accounts))
    }
}

/// `zero_unexpected_errors`: no unexpected status, transport error, timeout or broken refresh.
pub(crate) fn zero_unexpected_errors(
    report: &LoadReport,
    _: &LoadThresholds,
) -> Result<String, String> {
    let errors = &report.unexpected_errors;
    let summary = format!(
        "{} unexpected ({} statuses, {} transport errors, {} timeouts, {} undecodable refreshes) \
         over {} requests",
        errors.total,
        errors.unexpected_statuses,
        errors.transport_errors,
        errors.timeouts,
        errors.undecodable_session_answers,
        report.requests_sent
    );
    if errors.total == 0 && report.requests_sent > 0 {
        Ok(summary)
    } else {
        let templates: Vec<String> = report
            .templates
            .iter()
            .filter(|template| template.unexpected > 0)
            .map(|template| format!("{} {:?}", template.id, template.statuses))
            .collect();
        Err(format!("{summary}; templates: {}", templates.join(", ")))
    }
}

/// `p95_json_reads`: the reads' nearest-rank p95 is within its ceiling.
pub(crate) fn p95_json_reads(
    report: &LoadReport,
    limits: &LoadThresholds,
) -> Result<String, String> {
    percentile_case("JSON reads", &report.json_reads, limits.p95_json_read_ms)
}

/// `p95_json_writes`: the writes' nearest-rank p95 is within its ceiling.
pub(crate) fn p95_json_writes(
    report: &LoadReport,
    limits: &LoadThresholds,
) -> Result<String, String> {
    percentile_case("JSON writes", &report.json_writes, limits.p95_json_write_ms)
}

/// The report's half of `refresh_paced`: every refresh succeeded, none was throttled, and no
/// address passed its ceilings.
pub(crate) fn refresh_pacing(report: &LoadReport, _: &LoadThresholds) -> Result<String, String> {
    let refreshes = &report.refreshes;
    let ceilings = &report.per_address_ceilings;
    let busiest = report
        .addresses
        .iter()
        .map(|address| {
            format!(
                "{} {}/{} auth {}/{}",
                address.address,
                address.busiest_window_requests,
                ceilings.all_requests.max_requests,
                address.busiest_window_auth_requests,
                ceilings.auth_requests.max_requests
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let summary = format!(
        "{} of {} refreshes succeeded, statuses {:?}; busiest windows {busiest}",
        refreshes.succeeded, refreshes.attempted, refreshes.statuses
    );
    let within = report.addresses.iter().all(|address| {
        address.busiest_window_requests <= u64::from(ceilings.all_requests.max_requests)
            && address.busiest_window_auth_requests
                <= u64::from(ceilings.auth_requests.max_requests)
    });
    if refreshes.attempted > 0
        && refreshes.succeeded == refreshes.attempted
        && !refreshes.statuses.contains_key(&429)
        && within
    {
        Ok(summary)
    } else {
        Err(summary)
    }
}

fn percentile_case(class: &str, summary: &ClassSummary, ceiling_ms: f64) -> Result<String, String> {
    let text = format!(
        "{class}: p95 {} ms, p50 {} ms, max {} ms over {} expected answers",
        milliseconds(summary.p95_milliseconds),
        milliseconds(summary.p50_milliseconds),
        milliseconds(summary.max_milliseconds),
        summary.expected
    );
    match summary.p95_milliseconds {
        Some(p95) if summary.expected > 0 && p95 <= ceiling_ms => Ok(text),
        _ => Err(format!("{text}; needs at most {ceiling_ms} ms")),
    }
}

fn milliseconds(value: Option<f64>) -> String {
    value.map_or_else(|| "none".to_string(), |ms| format!("{ms:.1}"))
}
