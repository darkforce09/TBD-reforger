//! The catalogue of failpoints: every named point a call site may pass and a test may arm.
//!
//! **Role:** [`Failpoint`], one variant per point, its stable [`Failpoint::name`], and
//! [`CATALOGUE`], the list the failure suites walk so every point is exercised.
//! **Position:** `core::failpoints`, compiled only with the `failpoints` feature; the
//! `fail_point!` macro names a variant at each call site, and the registry keys armed entries by
//! it.
//! **Signals & state:** none; plain values.
//! **Invariants:** a `BeforeCommit` point sits inside the transaction, before its commit, so an
//! injected failure rolls every write back; an `AfterCommit` point sits after the commit, so an
//! injected failure leaves the writes committed and the caller answered with 500 (the response
//! is lost); a `BeforeEffect` or `AfterEffect` point brackets an external call the same way; the
//! names are the variant identifiers and are unique; [`CATALOGUE`] holds every variant except the
//! two that exist only in the library's unit-test build.

/// A named point on a commit or external-effect path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Failpoint {
    /// Refresh-token rotation, before the transaction that spends the presented token and issues
    /// its successor commits.
    SessionRotationBeforeCommit,
    /// Refresh-token rotation, after that transaction commits and before the new pair is
    /// answered.
    SessionRotationAfterCommit,
    /// Logout, before the transaction that revokes the session commits.
    SessionLogoutBeforeCommit,
    /// A participant's own reservation claim on an event mission, before the transaction that
    /// takes the place or seat commits.
    ReservationClaimBeforeCommit,
    /// A participant's own reservation claim on an event mission, after that transaction commits.
    ReservationClaimAfterCommit,
    /// A mission review decision (approve or reject), before its transaction commits.
    ReviewDecisionBeforeCommit,
    /// A mission deployment request, before its transaction commits.
    DeploymentRequestBeforeCommit,
    /// A mission deployment request, after its transaction commits.
    DeploymentRequestAfterCommit,
    /// A match results revision from the game runtime, before its transaction commits.
    ResultsRevisionBeforeCommit,
    /// A match results revision from the game runtime, after its transaction commits.
    ResultsRevisionAfterCommit,
    /// A fleet command claim by an executor, after its transaction commits, whether or not a
    /// command was claimed.
    FleetCommandClaimAfterCommit,
    /// A fleet command result report, before its transaction commits.
    FleetCommandResultBeforeCommit,
    /// A fleet command result report, after its transaction commits.
    FleetCommandResultAfterCommit,
    /// Audit publication, before the transaction that assigns publication sequences commits.
    AuditPublicationBeforeCommit,
    /// Audit delivery, before the snapshot that reads the publication bounds and the page after a
    /// stream's cursor; a failure there keeps the cursor and retries on the next wake.
    AuditDeliveryRead,
    /// A Discord membership reconciliation, before the REST call that reads the member's guild
    /// roles; the refresh lease and the request budget are already held.
    DiscordRoleSyncBeforeEffect,
    /// A Discord membership reconciliation, after Discord answered and before the observation or
    /// the failure is recorded under the refresh lease.
    DiscordRoleSyncAfterEffect,
    /// An Arma identity link confirmation, before its transaction commits.
    IdentityLinkConfirmBeforeCommit,
    /// A point only the registry's unit tests pass and arm, so they never touch a catalogue
    /// entry another unit test's code path may reach.
    #[cfg(test)]
    RegistryUnitTestFirst,
    /// The second point only the registry's unit tests pass and arm.
    #[cfg(test)]
    RegistryUnitTestSecond,
}

/// Every catalogue failpoint, in declaration order.
pub const CATALOGUE: [Failpoint; 18] = [
    Failpoint::SessionRotationBeforeCommit,
    Failpoint::SessionRotationAfterCommit,
    Failpoint::SessionLogoutBeforeCommit,
    Failpoint::ReservationClaimBeforeCommit,
    Failpoint::ReservationClaimAfterCommit,
    Failpoint::ReviewDecisionBeforeCommit,
    Failpoint::DeploymentRequestBeforeCommit,
    Failpoint::DeploymentRequestAfterCommit,
    Failpoint::ResultsRevisionBeforeCommit,
    Failpoint::ResultsRevisionAfterCommit,
    Failpoint::FleetCommandClaimAfterCommit,
    Failpoint::FleetCommandResultBeforeCommit,
    Failpoint::FleetCommandResultAfterCommit,
    Failpoint::AuditPublicationBeforeCommit,
    Failpoint::AuditDeliveryRead,
    Failpoint::DiscordRoleSyncBeforeEffect,
    Failpoint::DiscordRoleSyncAfterEffect,
    Failpoint::IdentityLinkConfirmBeforeCommit,
];

impl Failpoint {
    /// The variant identifier, as failure messages and test output name the point.
    pub const fn name(self) -> &'static str {
        match self {
            Self::SessionRotationBeforeCommit => "SessionRotationBeforeCommit",
            Self::SessionRotationAfterCommit => "SessionRotationAfterCommit",
            Self::SessionLogoutBeforeCommit => "SessionLogoutBeforeCommit",
            Self::ReservationClaimBeforeCommit => "ReservationClaimBeforeCommit",
            Self::ReservationClaimAfterCommit => "ReservationClaimAfterCommit",
            Self::ReviewDecisionBeforeCommit => "ReviewDecisionBeforeCommit",
            Self::DeploymentRequestBeforeCommit => "DeploymentRequestBeforeCommit",
            Self::DeploymentRequestAfterCommit => "DeploymentRequestAfterCommit",
            Self::ResultsRevisionBeforeCommit => "ResultsRevisionBeforeCommit",
            Self::ResultsRevisionAfterCommit => "ResultsRevisionAfterCommit",
            Self::FleetCommandClaimAfterCommit => "FleetCommandClaimAfterCommit",
            Self::FleetCommandResultBeforeCommit => "FleetCommandResultBeforeCommit",
            Self::FleetCommandResultAfterCommit => "FleetCommandResultAfterCommit",
            Self::AuditPublicationBeforeCommit => "AuditPublicationBeforeCommit",
            Self::AuditDeliveryRead => "AuditDeliveryRead",
            Self::DiscordRoleSyncBeforeEffect => "DiscordRoleSyncBeforeEffect",
            Self::DiscordRoleSyncAfterEffect => "DiscordRoleSyncAfterEffect",
            Self::IdentityLinkConfirmBeforeCommit => "IdentityLinkConfirmBeforeCommit",
            #[cfg(test)]
            Self::RegistryUnitTestFirst => "RegistryUnitTestFirst",
            #[cfg(test)]
            Self::RegistryUnitTestSecond => "RegistryUnitTestSecond",
        }
    }
}

#[cfg(test)]
#[path = "tests/catalogue.rs"]
mod tests;
