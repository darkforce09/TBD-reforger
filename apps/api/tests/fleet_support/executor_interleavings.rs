//! The generated executor interleavings of the fleet command ledger property: the host-agent
//! actions, the steps two executor credentials take, and the proptest strategies that draw them.
//!
//! **Role:** the input vocabulary of `command_executor_fencing_preserves_observed_outcomes`: which
//! commands a case enqueues and which claims, reports, lease lapses, queue expiries and
//! reconciliation passes follow, with the weights each is drawn at.
//! **Position:** mounted by `tests/fleet_command_properties.rs` with `#[path]`, beside the oracle
//! and the ledger world that execute what it generates; no other suite compiles it.
//! **Signals & state:** none; pure value types and strategies.
//! **Invariants:** executor indices stay below [`EXECUTORS`]; a case enqueues one to three
//! commands and runs one to fourteen steps; a [`Target::Command`] index is taken modulo the case's
//! command count by the executor of the step.

use fleet_wire_contract::FleetAction;
use proptest::prelude::*;

/// Host-agent credentials of the one server; generated steps name them by index.
pub(crate) const EXECUTORS: usize = 2;

/// The host-agent actions, with the oracle's own statement of which repeat safely and which change
/// the server process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HostAction {
    Start,
    Stop,
    Restart,
    ListPlayers,
}

impl HostAction {
    /// The wire action the command is enqueued with.
    pub(crate) fn fleet_action(self) -> FleetAction {
        match self {
            Self::Start => FleetAction::Start,
            Self::Stop => FleetAction::Stop,
            Self::Restart => FleetAction::Restart,
            Self::ListPlayers => FleetAction::ListPlayers,
        }
    }

    /// Whether running the action twice has the effect of running it once.
    pub(crate) fn idempotent(self) -> bool {
        self != Self::Restart
    }

    /// Whether the action changes the server process.
    pub(crate) fn process_changing(self) -> bool {
        self != Self::ListPlayers
    }
}

/// The fencing token an executor presents relative to the one it last received for the command
/// (the command's current token when it never claimed it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenSkew {
    Held,
    Behind,
    Ahead,
}

/// The executor that reports: the one holding the target's claim (else the last one that claimed
/// it, else the first credential), or a named credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reporter {
    Holder,
    Executor(usize),
}

/// The command a step acts on: the one claimed most recently in the case (else the first), or an
/// index into the case's commands modulo their count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    LatestClaim,
    Command(usize),
}

/// One generated step.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Step {
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
pub(crate) struct Interleaving {
    pub(crate) actions: Vec<HostAction>,
    pub(crate) steps: Vec<Step>,
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

/// The strategy every case of the property draws from.
pub(crate) fn interleaving_strategy() -> impl Strategy<Value = Interleaving> {
    (
        prop::collection::vec(action_strategy(), 1..=3),
        prop::collection::vec(step_strategy(), 1..=14),
    )
        .prop_map(|(actions, steps)| Interleaving { actions, steps })
}
