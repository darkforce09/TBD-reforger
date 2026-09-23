//! The claim loop: claim the next command, re-validate it, report `executing`, perform it,
//! report the result.
//!
//! The rules the loop keeps:
//!
//! - Nothing is performed without an acknowledged `executing` report. A transient failure of
//!   that report is retried before acting; a 409 `STALE_FENCING_TOKEN` (the claim was taken
//!   away) or any other refusal abandons the command without acting and without reporting it
//!   again.
//! - A command that fails re-validation, including one naming an action this host does not
//!   perform, is reported as failed straight from the claimed state, before any effect.
//! - The result report is retried with backoff until the API acknowledges it or answers 409.
//!   The effect itself is never repeated: when the outcome of a command that is not idempotent
//!   stays unreported, the ledger marks it indeterminate and an operator decides.
//! - Commands run one at a time. A shutdown request stops the loop between commands, so a
//!   command in progress is performed and reported first.

use std::future::Future;
use std::pin::pin;
use std::time::Duration;

use tracing::{Instrument, error, info, info_span, warn};

use super::ledger_api::{ClaimOutcome, LedgerApi, LedgerError};
use super::ledger_messages::{ClaimedFleetCommand, ExecutionResult};
use super::retry_backoff::{BackoffPolicy, JitteredBackoff};
use crate::action_verdict::ActionVerdict;
use crate::command_execution::{FleetActionExecutor, HostCommand};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedgerTimings {
    /// The wait before the next claim after the ledger had nothing for this server.
    pub poll_interval: Duration,
    /// Backoff between claims after a failed claim.
    pub claim_retry: BackoffPolicy,
    /// Backoff between attempts of an `executing` or result report.
    pub report_retry: BackoffPolicy,
}

impl LedgerTimings {
    /// Production timings around the configured poll interval.
    pub fn with_poll_interval(poll_interval: Duration) -> Self {
        Self {
            poll_interval,
            claim_retry: BackoffPolicy {
                initial: Duration::from_secs(1),
                maximum: Duration::from_secs(60),
            },
            report_retry: BackoffPolicy {
                initial: Duration::from_millis(500),
                maximum: Duration::from_secs(15),
            },
        }
    }
}

pub struct CommandLoop<E> {
    api: LedgerApi,
    executor: E,
    timings: LedgerTimings,
}

impl<E: FleetActionExecutor> CommandLoop<E> {
    pub fn new(api: LedgerApi, executor: E, timings: LedgerTimings) -> Self {
        Self {
            api,
            executor,
            timings,
        }
    }

    /// Claims and performs commands until `shutdown` completes.
    pub async fn run(&self, shutdown: impl Future<Output = ()>) {
        let mut shutdown = pin!(shutdown);
        let mut claim_backoff = JitteredBackoff::new(self.timings.claim_retry);
        let mut delay = Duration::ZERO;
        loop {
            tokio::select! {
                () = &mut shutdown => {
                    info!("shutdown requested; no further command is claimed");
                    return;
                }
                () = tokio::time::sleep(delay) => {}
            }
            delay = match self.api.claim().await {
                Ok(ClaimOutcome::NothingClaimable) => {
                    claim_backoff.reset();
                    self.timings.poll_interval
                }
                Ok(ClaimOutcome::Claimed(claimed)) => {
                    claim_backoff.reset();
                    self.process(claimed).await;
                    Duration::ZERO
                }
                Err(failure) => {
                    let retry_in = claim_backoff.next_delay();
                    if failure.is_transient() {
                        warn!(%failure, retry_in = ?retry_in, "claim failed");
                    } else {
                        error!(%failure, retry_in = ?retry_in, "claim refused");
                    }
                    retry_in
                }
            };
        }
    }

    async fn process(&self, claimed: ClaimedFleetCommand) {
        let span = info_span!(
            "fleet_command",
            command_id = %claimed.command_id,
            action = %claimed.action,
            fencing_token = claimed.fencing_token,
        );
        async {
            info!(
                server_id = %claimed.server_id,
                lease_expires_at = %claimed.lease_expires_at,
                "claimed"
            );
            let command = match HostCommand::from_claim(&claimed.action, &claimed.arguments) {
                Ok(command) => command,
                Err(refusal) => {
                    warn!(%refusal, "refused without acting");
                    let reason = format!("the host agent refused the command: {refusal}");
                    self.report_result(&claimed, ActionVerdict::failure(&reason, None))
                        .await;
                    return;
                }
            };
            if !self.report_executing(&claimed).await {
                return;
            }
            let verdict = self.executor.execute(&command).await;
            info!(
                succeeded = verdict.succeeded(),
                failure_reason = ?verdict.failure_reason(),
                "effect observed"
            );
            self.report_result(&claimed, verdict).await;
        }
        .instrument(span)
        .await;
    }

    /// True once the ledger acknowledged that the effect is starting; false when the command
    /// is abandoned without acting.
    async fn report_executing(&self, claimed: &ClaimedFleetCommand) -> bool {
        let mut backoff = JitteredBackoff::new(self.timings.report_retry);
        loop {
            match self
                .api
                .report_executing(claimed.command_id, claimed.fencing_token)
                .await
            {
                Ok(()) => {
                    info!("executing reported");
                    return true;
                }
                Err(failure) if failure.is_transient() => {
                    let retry_in = backoff.next_delay();
                    warn!(%failure, retry_in = ?retry_in, "executing report failed; retrying before acting");
                    tokio::time::sleep(retry_in).await;
                }
                Err(LedgerError::StaleFencingToken) => {
                    warn!(
                        "the claim was taken away (STALE_FENCING_TOKEN); abandoned without acting"
                    );
                    return false;
                }
                Err(failure) => {
                    error!(%failure, "executing report refused; abandoned without acting");
                    return false;
                }
            }
        }
    }

    /// Reports the verdict until the API acknowledges it or answers with a final refusal.
    async fn report_result(&self, claimed: &ClaimedFleetCommand, verdict: ActionVerdict) {
        let result = ExecutionResult::from_verdict(claimed.fencing_token, verdict);
        let mut backoff = JitteredBackoff::new(self.timings.report_retry);
        loop {
            match self.api.report_result(claimed.command_id, &result).await {
                Ok(()) => {
                    info!(succeeded = result.succeeded, "result reported");
                    return;
                }
                Err(failure) if failure.is_transient() => {
                    let retry_in = backoff.next_delay();
                    warn!(%failure, retry_in = ?retry_in, "result report failed; retrying");
                    tokio::time::sleep(retry_in).await;
                }
                Err(LedgerError::StaleFencingToken) => {
                    warn!(
                        "the claim was taken away (STALE_FENCING_TOKEN); the result is not reported again"
                    );
                    return;
                }
                Err(failure) => {
                    error!(%failure, "result report refused; the result is not reported again");
                    return;
                }
            }
        }
    }
}
