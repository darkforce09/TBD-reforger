//! The thirteen cases the `staging_discord` receipt declares, and the scenarios its observations
//! name.
//!
//! **Role:** the case names every step module maps its effects to, the declared case list in
//! receipt order (the two cases no staging account can run carry their missing dependency), and
//! the mapping from recorded cases to the judge's scenario names.
//!
//! **Position:** read by the step modules of `discord_procedure/`, by `DiscordProcedure::plan`
//! and by `DiscordProcedure::observations` in `mod.rs`.
//!
//! **Signals & state:** none; constants and pure functions.
//!
//! **Invariants:** eleven cases run and two are `NOT RUN (missing: test subject Discord account)`,
//! because the operator's own account carries every scenario and its Command Staff role is never
//! touched; a scenario is named only when its case is `ok`, so a failing or not-run case never
//! reaches the judge as observed.

use crate::error::Result;

use crate::procedure_receipts::{CaseName, CaseStatus, RecordedCase};
use crate::procedure_runner::step::DeclaredCase;

/// A partner-only registration enrols the partner snapshot and then registers.
pub(super) const PARTNER_MEMBERSHIP: &str = "partner_membership";
/// Losing the partner role releases the reservation `eligibility_lost`, audited.
pub(super) const ELIGIBILITY_RELEASE: &str = "eligibility_release";
/// The release follows the first bot read that shows the role gone within 60 s.
pub(super) const PROPAGATION_WITHIN_60_SECONDS: &str = "partner_role_propagation_within_60_seconds";
/// Discord unreachable: `unavailable` outcomes, `last_error` set, membership unchanged.
pub(super) const NETWORK_OUTAGE: &str = "network_outage";
/// The site warns that the membership is stale during the outage.
pub(super) const STALENESS_WARNING: &str = "staleness_warning";
/// The cached role still applies during the outage.
pub(super) const CACHED_GRACE: &str = "cached_grace";
/// The site keeps serving during the outage.
pub(super) const NON_BLOCKING_DURING_OUTAGE: &str = "non_blocking_during_outage";
/// An administrator extends the grace of a snapshot past its window.
pub(super) const ADMIN_OVERRIDE: &str = "admin_override";
/// Discord reachable again: a fresh verification, the partner role back.
pub(super) const OUTAGE_RECOVERY: &str = "outage_recovery";
/// A spent Get Guild Member bucket: the API's refresh meets a 429 and backs off.
pub(super) const RATE_LIMIT: &str = "rate_limit";
/// The refresh after the rate limit verifies again.
pub(super) const RATE_LIMIT_RECOVERY: &str = "rate_limit_recovery";
/// A Discord role change demotes a site role; needs an account other than the operator's.
pub(super) const ROLE_DEMOTION: &str = "role_demotion";
/// Leaving the main guild turns the account into a guest; needs an account other than the
/// operator's.
pub(super) const DEPARTURE_TO_GUEST: &str = "departure_to_guest";

/// The dependency the two subject-account cases lack.
pub(super) const MISSING_TEST_SUBJECT: &str = "test subject Discord account";

/// The cases the procedure runs, in receipt order.
const RUN_CASES: [&str; 11] = [
    PARTNER_MEMBERSHIP,
    ELIGIBILITY_RELEASE,
    PROPAGATION_WITHIN_60_SECONDS,
    NETWORK_OUTAGE,
    STALENESS_WARNING,
    CACHED_GRACE,
    NON_BLOCKING_DURING_OUTAGE,
    ADMIN_OVERRIDE,
    OUTAGE_RECOVERY,
    RATE_LIMIT,
    RATE_LIMIT_RECOVERY,
];

/// The cases no staging account can run.
const NOT_RUN_CASES: [&str; 2] = [ROLE_DEMOTION, DEPARTURE_TO_GUEST];

/// Every declared case: the eleven that run, then the two that cannot.
pub(super) fn declared_cases() -> Result<Vec<DeclaredCase>> {
    let mut cases = Vec::with_capacity(RUN_CASES.len() + NOT_RUN_CASES.len());
    for name in RUN_CASES {
        cases.push(DeclaredCase::runs(name)?);
    }
    for name in NOT_RUN_CASES {
        cases.push(DeclaredCase::not_run(name, MISSING_TEST_SUBJECT)?);
    }
    Ok(cases)
}

/// `name` as a case name.
pub(super) fn case(name: &str) -> Result<CaseName> {
    CaseName::new(name)
}

/// The scenario names the observations carry: every case that ended `ok`, in receipt order.
pub(super) fn scenarios(cases: &[RecordedCase]) -> Vec<String> {
    cases
        .iter()
        .filter(|case| matches!(case.status, CaseStatus::Ok))
        .map(|case| case.name.as_str().to_string())
        .collect()
}
