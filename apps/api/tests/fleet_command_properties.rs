//! Generated executor interleavings hold the production fleet command ledger to an independent
//! oracle of claims, fencing tokens, leases and observed outcomes.
//!
//! **Role:** the property `command_executor_fencing_preserves_observed_outcomes`: every case
//! enqueues fresh host-agent commands and runs a generated interleaving of claims by two executor
//! credentials, `executing` and outcome reports under held, stale or skewed fencing tokens, lease
//! lapses, queue expiry and reconciliation passes through the production `enqueue_command`,
//! `claim_next_command`, `mark_executing`, `record_result` and `reconcile_fleet_commands`. Every
//! answer or refusal equals the oracle's and every command row equals the oracle's after every
//! step; the case then drains every command to a terminal state, has each executor present its
//! last token for every command (all refused), and checks the audit history: each observed outcome
//! is recorded once and is the one stored, and a non-idempotent command starts its effect at most
//! once.
//! **Position:** a `db test-it` binary over its own database, built on one
//! `event_eligibility_support::Fixture` (the requesting administrator) and `fleet_support`'s
//! server registration and credential issuance.
//! **Signals & state:** one database, one server and two executor credentials serve every case;
//! each case owns the commands it enqueues. Every passing case ends with its commands terminal, so
//! the next case's claims and the global reconciliation pass see only that case's commands; a case
//! first retires any command a failed earlier case left active (none in a passing run). The
//! coverage tally is test-local. Every step ends its transaction, by commit or by an awaited
//! rollback, before the next step runs: a dropped transaction's rollback waits on the pool, and the
//! row lock it still holds would hide that command from the next claim's `SKIP LOCKED` read.
//! **Invariants:** a report is accepted only with the command's current fencing token from the
//! credential holding the claim; a claim lapsed before the effect started returns to the queue
//! under the next token; a lapse after a non-idempotent effect started makes the command
//! indeterminate, and nothing claims or reports it again; a terminal command accepts no report.

mod common;
mod event_eligibility_support;
mod fleet_support;

use std::cell::Cell;
use std::collections::HashMap;

use api::core::error_handling::api_error::ApiError;
use api::server_infrastructure::services::fleet_commands::command_ledger::enqueue_command;
use api::server_infrastructure::services::fleet_commands::command_reconciliation::reconcile_fleet_commands;
use api::server_infrastructure::services::fleet_commands::executor_claims::{
    claim_next_command, mark_executing, record_result,
};
use api::server_infrastructure::services::machine_credentials::{
    MachineCaller, authenticate_machine,
};
use event_eligibility_support::{EventShape, Fixture};
use fleet_support::{credential, register_server};
use fleet_wire_contract::FleetAction;
use fleet_wire_contract::executor_messages::ExecutionResult;
use fleet_wire_contract::operator_messages::FleetCommandRequest;
use proptest::prelude::*;
use proptest::test_runner::TestCaseResult;
use serde_json::{Value, json};
use uuid::Uuid;

/// Cases the property executes.
const CASES: u32 = 256;

/// Host-agent credentials of the one server; generated steps name them by index.
const EXECUTORS: usize = 2;

/// The failure reason every failed outcome report carries.
const FAILURE_REASON: &str = "the property executor observed a failure";

/// The host-agent actions, with the oracle's own statement of which repeat safely and which change
/// the server process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HostAction {
    Start,
    Stop,
    Restart,
    ListPlayers,
}

impl HostAction {
    fn fleet_action(self) -> FleetAction {
        match self {
            Self::Start => FleetAction::Start,
            Self::Stop => FleetAction::Stop,
            Self::Restart => FleetAction::Restart,
            Self::ListPlayers => FleetAction::ListPlayers,
        }
    }

    fn idempotent(self) -> bool {
        self != Self::Restart
    }

    fn process_changing(self) -> bool {
        self != Self::ListPlayers
    }
}

/// The fencing token an executor presents relative to the one it last received for the command
/// (the command's current token when it never claimed it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenSkew {
    Held,
    Behind,
    Ahead,
}

/// The executor that reports: the one holding the target's claim (else the last one that claimed
/// it, else the first credential), or a named credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reporter {
    Holder,
    Executor(usize),
}

/// The command a step acts on: the one claimed most recently in the case (else the first), or an
/// index into the case's commands modulo their count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    LatestClaim,
    Command(usize),
}

/// One generated step.
#[derive(Debug, Clone, Copy)]
enum Step {
    Claim {
        executor: usize,
    },
    ReportExecuting {
        reporter: Reporter,
        target: Target,
        skew: TokenSkew,
    },
    ReportOutcome {
        reporter: Reporter,
        target: Target,
        skew: TokenSkew,
        succeeded: bool,
    },
    LapseLease {
        target: Target,
    },
    ExpireQueue {
        target: Target,
    },
    Reconcile,
}

/// A generated case: the commands to enqueue, in order, and the steps that follow.
#[derive(Debug, Clone)]
struct Interleaving {
    actions: Vec<HostAction>,
    steps: Vec<Step>,
}

fn action_strategy() -> impl Strategy<Value = HostAction> {
    prop_oneof![
        1 => Just(HostAction::Start),
        1 => Just(HostAction::Stop),
        2 => Just(HostAction::Restart),
        2 => Just(HostAction::ListPlayers),
    ]
}

fn skew_strategy() -> impl Strategy<Value = TokenSkew> {
    prop_oneof![
        4 => Just(TokenSkew::Held),
        1 => Just(TokenSkew::Behind),
        1 => Just(TokenSkew::Ahead),
    ]
}

fn reporter_strategy() -> impl Strategy<Value = Reporter> {
    prop_oneof![
        3 => Just(Reporter::Holder),
        1 => (0..EXECUTORS).prop_map(Reporter::Executor),
    ]
}

fn target_strategy() -> impl Strategy<Value = Target> {
    prop_oneof![
        3 => Just(Target::LatestClaim),
        1 => (0_usize..3).prop_map(Target::Command),
    ]
}

fn step_strategy() -> impl Strategy<Value = Step> {
    prop_oneof![
        4 => (0..EXECUTORS).prop_map(|executor| Step::Claim { executor }),
        3 => (reporter_strategy(), target_strategy(), skew_strategy()).prop_map(
            |(reporter, target, skew)| Step::ReportExecuting { reporter, target, skew }
        ),
        3 => (reporter_strategy(), target_strategy(), skew_strategy(), any::<bool>()).prop_map(
            |(reporter, target, skew, succeeded)| Step::ReportOutcome {
                reporter,
                target,
                skew,
                succeeded,
            }
        ),
        2 => target_strategy().prop_map(|target| Step::LapseLease { target }),
        1 => target_strategy().prop_map(|target| Step::ExpireQueue { target }),
        2 => Just(Step::Reconcile),
    ]
}

fn interleaving_strategy() -> impl Strategy<Value = Interleaving> {
    (
        prop::collection::vec(action_strategy(), 1..=3),
        prop::collection::vec(step_strategy(), 1..=14),
    )
        .prop_map(|(actions, steps)| Interleaving { actions, steps })
}

/// A command's lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Queued,
    Claimed,
    Executing,
    Succeeded,
    Failed,
    Expired,
    Indeterminate,
}

impl Phase {
    fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Claimed => "claimed",
            Self::Executing => "executing",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Expired => "expired",
            Self::Indeterminate => "indeterminate",
        }
    }

    fn is_active(self) -> bool {
        matches!(self, Self::Claimed | Self::Executing)
    }

    fn is_terminal(self) -> bool {
        !matches!(self, Self::Queued | Self::Claimed | Self::Executing)
    }
}

/// What one step answers: a claim, a receipt, a refusal, or a reconciliation pass.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Observed {
    Claimed {
        command: Uuid,
        token: i64,
    },
    NothingToClaim,
    Receipt {
        command: Uuid,
        state: String,
        attempts: i32,
    },
    Refused {
        status: u16,
        code: String,
    },
    Reconciled {
        expired: usize,
        requeued: usize,
        indeterminate: usize,
    },
    Done,
}

fn refused(code: &str) -> Observed {
    Observed::Refused {
        status: 409,
        code: code.to_owned(),
    }
}

/// One command as the ledger rules define it.
struct CommandModel {
    id: Uuid,
    action: HostAction,
    phase: Phase,
    token: i64,
    holder: Option<usize>,
    last_claimant: Option<usize>,
    attempts: i32,
    lease_lapsed: bool,
    queue_expired: bool,
    recorded_report: Option<usize>,
    executions: i64,
}

impl CommandModel {
    fn receipt(&self) -> Observed {
        Observed::Receipt {
            command: self.id,
            state: self.phase.as_str().to_owned(),
            attempts: self.attempts,
        }
    }
}

/// The oracle: every command of the case, the claim order, and the last token each executor
/// received for each command.
struct LedgerModel {
    commands: Vec<CommandModel>,
    queue_order: Vec<usize>,
    held: Vec<HashMap<usize, i64>>,
    latest_claim: Option<usize>,
}

impl LedgerModel {
    fn new(actions: &[HostAction], ids: &[Uuid], queue_order: Vec<usize>) -> Self {
        let commands = actions
            .iter()
            .zip(ids)
            .map(|(action, id)| CommandModel {
                id: *id,
                action: *action,
                phase: Phase::Queued,
                token: 0,
                holder: None,
                last_claimant: None,
                attempts: 0,
                lease_lapsed: false,
                queue_expired: false,
                recorded_report: None,
                executions: 0,
            })
            .collect();
        Self {
            commands,
            queue_order,
            held: vec![HashMap::new(); EXECUTORS],
            latest_claim: None,
        }
    }

    fn target(&self, target: Target) -> usize {
        match target {
            Target::LatestClaim => self.latest_claim.unwrap_or(0),
            Target::Command(index) => index % self.commands.len(),
        }
    }

    fn reporter(&self, reporter: Reporter, index: usize) -> usize {
        let command = &self.commands[index];
        match reporter {
            Reporter::Holder => command.holder.or(command.last_claimant).unwrap_or(0),
            Reporter::Executor(executor) => executor,
        }
    }

    fn presented_token(&self, executor: usize, command: usize, skew: TokenSkew) -> i64 {
        let base = self.held[executor]
            .get(&command)
            .copied()
            .unwrap_or(self.commands[command].token);
        let presented = match skew {
            TokenSkew::Held => base,
            TokenSkew::Behind => base - 1,
            TokenSkew::Ahead => base + 1,
        };
        presented.max(1)
    }

    fn claim(&mut self, executor: usize) -> Observed {
        let busy = self
            .commands
            .iter()
            .any(|command| command.action.process_changing() && command.phase.is_active());
        let next = self.queue_order.iter().copied().find(|index| {
            let command = &self.commands[*index];
            command.phase == Phase::Queued
                && !command.queue_expired
                && !(command.action.process_changing() && busy)
        });
        let Some(index) = next else {
            return Observed::NothingToClaim;
        };
        let command = &mut self.commands[index];
        command.phase = Phase::Claimed;
        command.token += 1;
        command.holder = Some(executor);
        command.last_claimant = Some(executor);
        command.attempts += 1;
        command.lease_lapsed = false;
        self.held[executor].insert(index, command.token);
        self.latest_claim = Some(index);
        Observed::Claimed {
            command: command.id,
            token: command.token,
        }
    }

    fn is_fenced(&self, executor: usize, command: usize, token: i64) -> bool {
        let command = &self.commands[command];
        command.token != token || command.holder != Some(executor)
    }

    fn report_executing(&mut self, executor: usize, index: usize, token: i64) -> Observed {
        if self.is_fenced(executor, index, token) {
            return refused("STALE_FENCING_TOKEN");
        }
        let command = &mut self.commands[index];
        match command.phase {
            Phase::Executing => {}
            Phase::Claimed => {
                command.phase = Phase::Executing;
                command.lease_lapsed = false;
                command.executions += 1;
            }
            _ => return refused("COMMAND_NOT_CLAIMED"),
        }
        command.receipt()
    }

    fn report_outcome(
        &mut self,
        executor: usize,
        index: usize,
        report: (i64, bool, usize),
    ) -> Observed {
        let (token, succeeded, marker) = report;
        if self.is_fenced(executor, index, token) {
            return refused("STALE_FENCING_TOKEN");
        }
        let command = &mut self.commands[index];
        let allowed =
            command.phase == Phase::Executing || (!succeeded && command.phase == Phase::Claimed);
        if !allowed {
            return refused("COMMAND_NOT_EXECUTING");
        }
        command.phase = if succeeded {
            Phase::Succeeded
        } else {
            Phase::Failed
        };
        command.holder = None;
        command.lease_lapsed = false;
        command.recorded_report = Some(marker);
        command.receipt()
    }

    fn lapse(&mut self, index: usize) -> Observed {
        let command = &mut self.commands[index];
        if command.phase.is_active() {
            command.lease_lapsed = true;
        }
        Observed::Done
    }

    fn expire(&mut self, index: usize) -> Observed {
        self.commands[index].queue_expired = true;
        Observed::Done
    }

    fn reconcile(&mut self) -> Observed {
        let (mut expired, mut requeued, mut indeterminate) = (0, 0, 0);
        for command in &mut self.commands {
            let due = (command.phase == Phase::Queued && command.queue_expired)
                || (command.phase.is_active() && command.lease_lapsed);
            if !due {
                continue;
            }
            command.phase = if command.phase == Phase::Executing && !command.action.idempotent() {
                indeterminate += 1;
                Phase::Indeterminate
            } else if command.queue_expired {
                expired += 1;
                Phase::Expired
            } else {
                requeued += 1;
                Phase::Queued
            };
            command.holder = None;
            command.lease_lapsed = false;
        }
        Observed::Reconciled {
            expired,
            requeued,
            indeterminate,
        }
    }
}

fn refusal(error: ApiError) -> Observed {
    Observed::Refused {
        status: error.status.as_u16(),
        code: error
            .details
            .as_ref()
            .and_then(|details| details["code"].as_str())
            .unwrap_or_default()
            .to_owned(),
    }
}

/// The refusal of a step whose transaction is rolled back before the next step runs.
async fn refused_after_rollback(
    transaction: sqlx::Transaction<'_, sqlx::Postgres>,
    error: ApiError,
) -> Observed {
    transaction
        .rollback()
        .await
        .expect("roll back a refused step");
    refusal(error)
}

/// The outcome object of report `marker`, so the stored outcome names the report it came from.
fn outcome_of(marker: usize) -> serde_json::Map<String, Value> {
    let Value::Object(outcome) = json!({ "report": marker }) else {
        unreachable!("a JSON object literal is an object")
    };
    outcome
}

/// A command row: `(state, fencing_token, claimed_by, attempts)`.
type CommandRow = (String, i64, Option<Uuid>, i32);

/// The server, its executors and the requesting administrator every case shares.
struct LedgerWorld {
    fixture: Fixture,
    server: Uuid,
    executors: Vec<MachineCaller>,
}

impl LedgerWorld {
    async fn open() -> Self {
        let fixture = Fixture::new(
            "fleet_command_properties",
            EventShape {
                max_slots: 0,
                missions: &[&["Alpha"]],
            },
        )
        .await;
        let server = register_server(&fixture, "Fencing property host").await;
        let mut executors = Vec::with_capacity(EXECUTORS);
        for _ in 0..EXECUTORS {
            let secret = credential(&fixture, server, "host_agent").await;
            let caller = authenticate_machine(fixture.pool(), &secret)
                .await
                .expect("an issued host-agent credential authenticates");
            executors.push(caller);
        }
        Self {
            fixture,
            server,
            executors,
        }
    }

    fn pool(&self) -> &sqlx::PgPool {
        self.fixture.pool()
    }

    /// Drive every command a failed earlier case left active to a terminal state.
    async fn retire_leftovers(&self) {
        sqlx::query(
            "UPDATE fleet_commands SET expires_at = requested_at + interval '1 microsecond',
                 lease_expires_at = CASE WHEN state IN ('claimed', 'executing')
                     THEN clock_timestamp() - interval '1 second' ELSE lease_expires_at END
             WHERE server_id = $1 AND state IN ('queued', 'claimed', 'executing')",
        )
        .bind(self.server)
        .execute(self.pool())
        .await
        .expect("lapse leftover commands");
        reconcile_fleet_commands(self.pool())
            .await
            .expect("retire leftover commands");
    }

    async fn enqueue(&self, action: HostAction) -> Uuid {
        let mut transaction = self.pool().begin().await.expect("begin enqueue");
        let request = FleetCommandRequest {
            action: action.fleet_action(),
            arguments: serde_json::Map::new(),
        };
        let receipt = enqueue_command(
            &mut transaction,
            self.server,
            &request,
            &self.fixture.admin.id,
        )
        .await
        .expect("enqueue a generated command");
        transaction.commit().await.expect("commit enqueue");
        receipt.id
    }

    /// The case's command indexes in the order claims consider them.
    async fn queue_order(&self, ids: &[Uuid]) -> Vec<usize> {
        let ordered: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM fleet_commands WHERE id = ANY($1) ORDER BY requested_at, id",
        )
        .bind(ids)
        .fetch_all(self.pool())
        .await
        .expect("read the queue order");
        ordered
            .iter()
            .map(|id| ids.iter().position(|candidate| candidate == id).unwrap())
            .collect()
    }

    async fn claim(&self, executor: usize) -> Observed {
        let mut transaction = self.pool().begin().await.expect("begin claim");
        match claim_next_command(
            &mut transaction,
            &self.executors[executor],
            None,
            &self.fixture.main_guild,
        )
        .await
        {
            Ok(claimed) => {
                transaction.commit().await.expect("commit claim");
                claimed.map_or(Observed::NothingToClaim, |command| Observed::Claimed {
                    command: command.command_id,
                    token: command.fencing_token,
                })
            }
            Err(error) => refused_after_rollback(transaction, error).await,
        }
    }

    async fn report_executing(&self, executor: usize, command: Uuid, token: i64) -> Observed {
        let mut transaction = self.pool().begin().await.expect("begin executing report");
        match mark_executing(&mut transaction, &self.executors[executor], command, token).await {
            Ok(receipt) => {
                transaction.commit().await.expect("commit executing report");
                Observed::Receipt {
                    command: receipt.id,
                    state: receipt.state,
                    attempts: receipt.attempts,
                }
            }
            Err(error) => refused_after_rollback(transaction, error).await,
        }
    }

    async fn report_outcome(
        &self,
        executor: usize,
        command: Uuid,
        (token, succeeded, marker): (i64, bool, usize),
    ) -> Observed {
        let result = ExecutionResult {
            fencing_token: token,
            succeeded,
            outcome: Some(outcome_of(marker)),
            failure_reason: (!succeeded).then(|| FAILURE_REASON.to_owned()),
        };
        let mut transaction = self.pool().begin().await.expect("begin outcome report");
        match record_result(
            &mut transaction,
            &self.executors[executor],
            command,
            &result,
        )
        .await
        {
            Ok(receipt) => {
                transaction.commit().await.expect("commit outcome report");
                Observed::Receipt {
                    command: receipt.id,
                    state: receipt.state,
                    attempts: receipt.attempts,
                }
            }
            Err(error) => refused_after_rollback(transaction, error).await,
        }
    }

    /// Time passes beyond the lease of an active claim.
    async fn lapse(&self, command: Uuid) -> Observed {
        sqlx::query(
            "UPDATE fleet_commands SET lease_expires_at = clock_timestamp() - interval '1 second'
             WHERE id = $1 AND state IN ('claimed', 'executing')",
        )
        .bind(command)
        .execute(self.pool())
        .await
        .expect("lapse the lease");
        Observed::Done
    }

    /// Time passes beyond the command's queued lifetime; the claim order is unchanged.
    async fn expire(&self, command: Uuid) -> Observed {
        sqlx::query(
            "UPDATE fleet_commands SET expires_at = requested_at + interval '1 microsecond'
             WHERE id = $1",
        )
        .bind(command)
        .execute(self.pool())
        .await
        .expect("expire the queued lifetime");
        Observed::Done
    }

    async fn reconcile(&self) -> Observed {
        match reconcile_fleet_commands(self.pool()).await {
            Ok(outcome) => Observed::Reconciled {
                expired: outcome.expired,
                requeued: outcome.requeued,
                indeterminate: outcome.indeterminate,
            },
            Err(error) => refusal(error),
        }
    }

    async fn rows(&self, model: &LedgerModel) -> Vec<CommandRow> {
        let mut rows = Vec::with_capacity(model.commands.len());
        for command in &model.commands {
            rows.push(
                sqlx::query_as(
                    "SELECT state, fencing_token, claimed_by, attempts FROM fleet_commands
                     WHERE id = $1",
                )
                .bind(command.id)
                .fetch_one(self.pool())
                .await
                .expect("read a command row"),
            );
        }
        rows
    }

    fn expected_rows(&self, model: &LedgerModel) -> Vec<CommandRow> {
        model
            .commands
            .iter()
            .map(|command| {
                (
                    command.phase.as_str().to_owned(),
                    command.token,
                    command
                        .holder
                        .map(|executor| self.executors[executor].credential_id),
                    command.attempts,
                )
            })
            .collect()
    }
}

/// Steps observed per class over the whole run; the property fails if any class stays zero.
#[derive(Default)]
struct Coverage {
    recorded_outcomes: Cell<u32>,
    stale_holder_refusals: Cell<u32>,
    skewed_token_refusals: Cell<u32>,
    foreign_executor_refusals: Cell<u32>,
    reclaims_after_requeue: Cell<u32>,
    indeterminate_commands: Cell<u32>,
    uncertain_reports_refused: Cell<u32>,
}

fn bump(cell: &Cell<u32>) {
    cell.set(cell.get() + 1);
}

impl Coverage {
    fn tally(&self) -> [(&'static str, u32); 7] {
        [
            ("recorded_outcomes", self.recorded_outcomes.get()),
            ("stale_holder_refusals", self.stale_holder_refusals.get()),
            ("skewed_token_refusals", self.skewed_token_refusals.get()),
            (
                "foreign_executor_refusals",
                self.foreign_executor_refusals.get(),
            ),
            ("reclaims_after_requeue", self.reclaims_after_requeue.get()),
            ("indeterminate_commands", self.indeterminate_commands.get()),
            (
                "uncertain_reports_refused",
                self.uncertain_reports_refused.get(),
            ),
        ]
    }

    /// Classify a report step before the model applies it.
    fn report(&self, model: &LedgerModel, executor: usize, index: usize, token: i64) {
        let command = &model.commands[index];
        if !model.is_fenced(executor, index, token) {
            return;
        }
        if command.phase == Phase::Indeterminate {
            bump(&self.uncertain_reports_refused);
        }
        match model.held[executor].get(&index) {
            Some(held) if *held == token => bump(&self.stale_holder_refusals),
            Some(_) => bump(&self.skewed_token_refusals),
            None => bump(&self.foreign_executor_refusals),
        }
    }

    /// Classify an observation both sides agreed on.
    fn observed(&self, observed: &Observed) {
        match observed {
            Observed::Claimed { token, .. } if *token > 1 => bump(&self.reclaims_after_requeue),
            Observed::Receipt { state, .. } if state == "succeeded" || state == "failed" => {
                bump(&self.recorded_outcomes)
            }
            Observed::Reconciled { indeterminate, .. } => {
                for _ in 0..*indeterminate {
                    bump(&self.indeterminate_commands);
                }
            }
            _ => {}
        }
    }
}

/// Run one step through the model and through production; answer both observations.
async fn perform(
    world: &LedgerWorld,
    model: &mut LedgerModel,
    step: Step,
    marker: usize,
    coverage: &Coverage,
) -> (Observed, Observed) {
    match step {
        Step::Claim { executor } => (model.claim(executor), world.claim(executor).await),
        Step::ReportExecuting {
            reporter,
            target,
            skew,
        } => {
            let index = model.target(target);
            let executor = model.reporter(reporter, index);
            let token = model.presented_token(executor, index, skew);
            coverage.report(model, executor, index, token);
            let id = model.commands[index].id;
            (
                model.report_executing(executor, index, token),
                world.report_executing(executor, id, token).await,
            )
        }
        Step::ReportOutcome {
            reporter,
            target,
            skew,
            succeeded,
        } => {
            let index = model.target(target);
            let executor = model.reporter(reporter, index);
            let token = model.presented_token(executor, index, skew);
            coverage.report(model, executor, index, token);
            let id = model.commands[index].id;
            let report = (token, succeeded, marker);
            (
                model.report_outcome(executor, index, report),
                world.report_outcome(executor, id, report).await,
            )
        }
        Step::LapseLease { target } => {
            let index = model.target(target);
            (
                model.lapse(index),
                world.lapse(model.commands[index].id).await,
            )
        }
        Step::ExpireQueue { target } => {
            let index = model.target(target);
            (
                model.expire(index),
                world.expire(model.commands[index].id).await,
            )
        }
        Step::Reconcile => (model.reconcile(), world.reconcile().await),
    }
}

/// The audit history and stored outcome of every command agree with what was observed.
async fn check_history(world: &LedgerWorld, model: &LedgerModel) -> TestCaseResult {
    for command in &model.commands {
        prop_assert!(command.phase.is_terminal(), "{:?} stays active", command.id);
        let audits: Vec<(String, i64)> = sqlx::query_as(
            "SELECT action, count(*) FROM audit_logs
             WHERE target_type = 'fleet_command' AND target_id = $1 GROUP BY action",
        )
        .bind(command.id.to_string())
        .fetch_all(world.pool())
        .await
        .expect("read the command's audit rows");
        let audits: HashMap<String, i64> = audits.into_iter().collect();
        let count = |action: &str| audits.get(action).copied().unwrap_or(0);
        let recorded = count("server.command_succeeded") + count("server.command_failed");
        prop_assert_eq!(recorded, i64::from(command.recorded_report.is_some()));
        prop_assert_eq!(count("server.command_claimed"), i64::from(command.attempts));
        prop_assert_eq!(count("server.command_executing"), command.executions);
        if !command.action.idempotent() {
            prop_assert!(
                count("server.command_executing") <= 1,
                "a non-idempotent command started its effect twice"
            );
        }
        let stored: Option<Value> =
            sqlx::query_scalar("SELECT outcome FROM fleet_commands WHERE id = $1")
                .bind(command.id)
                .fetch_one(world.pool())
                .await
                .expect("read the stored outcome");
        prop_assert_eq!(
            stored,
            command
                .recorded_report
                .map(|marker| Value::Object(outcome_of(marker)))
        );
    }
    Ok(())
}

async fn run_interleaving(
    world: &LedgerWorld,
    case: &Interleaving,
    coverage: &Coverage,
) -> TestCaseResult {
    world.retire_leftovers().await;
    let mut ids = Vec::with_capacity(case.actions.len());
    for action in &case.actions {
        ids.push(world.enqueue(*action).await);
    }
    let queue_order = world.queue_order(&ids).await;
    let mut model = LedgerModel::new(&case.actions, &ids, queue_order);
    prop_assert_eq!(world.rows(&model).await, world.expected_rows(&model));

    // The generated steps, then a drain that lets every lease and queued lifetime lapse, then
    // each executor presenting its last token (or the current one) for every terminal command.
    let mut steps = case.steps.clone();
    for command in 0..ids.len() {
        let target = Target::Command(command);
        steps.push(Step::ExpireQueue { target });
        steps.push(Step::LapseLease { target });
    }
    steps.push(Step::Reconcile);
    let drained = steps.len();
    for executor in 0..EXECUTORS {
        for command in 0..ids.len() {
            let (reporter, target) = (Reporter::Executor(executor), Target::Command(command));
            steps.push(Step::ReportExecuting {
                reporter,
                target,
                skew: TokenSkew::Held,
            });
            steps.push(Step::ReportOutcome {
                reporter,
                target,
                skew: TokenSkew::Held,
                succeeded: true,
            });
        }
    }

    for (marker, step) in steps.into_iter().enumerate() {
        let (expected, observed) = perform(world, &mut model, step, marker, coverage).await;
        prop_assert_eq!(&observed, &expected, "step {} {:?}", marker, step);
        coverage.observed(&observed);
        if marker >= drained {
            prop_assert!(
                matches!(observed, Observed::Refused { .. }),
                "a terminal command accepted {:?}",
                step
            );
        }
        prop_assert_eq!(
            world.rows(&model).await,
            world.expected_rows(&model),
            "rows after step {} {:?}",
            marker,
            step
        );
    }
    check_history(world, &model).await
}

#[test]
fn command_executor_fencing_preserves_observed_outcomes() {
    let runtime = tokio::runtime::Runtime::new().expect("build the test runtime");
    let world = runtime.block_on(LedgerWorld::open());
    let coverage = Coverage::default();
    common::property_evidence::run_property(
        "command_executor_fencing_preserves_observed_outcomes",
        CASES,
        &interleaving_strategy(),
        |case| runtime.block_on(run_interleaving(&world, &case, &coverage)),
    );
    let tally = coverage.tally();
    println!("fencing-coverage: {tally:?}");
    for (class, count) in tally {
        assert!(count > 0, "no generated step exercised {class}");
    }
}
