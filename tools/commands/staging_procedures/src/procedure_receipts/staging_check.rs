//! The three recorded staging checks and the limits a passing run of each must keep.
//!
//! **Role:** names each check, the files its receipt is written as, the time a run may take and
//! the fewest passing cases its receipt may report.
//!
//! **Position:** every procedure names its [`StagingCheck`]; `recording_session.rs` reads the
//! limits when it judges a run; `run_identity.rs` derives the run folder from the id.
//!
//! **Signals & state:** none; constants.
//!
//! **Invariants:** an id is `staging_` followed by `[a-z]+`, so the passing marker
//! `<id>: PASS` holds no backslash and never overlaps itself; the time limit lies above every
//! procedure's own hard stop.

/// How long a recorded run may take, in seconds, before it fails: two hours.
const RUN_TIME_LIMIT_SECONDS: u64 = 7_200;

/// The three staging checks, each recorded by one procedure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StagingCheck {
    /// `staging_fleet`: five game servers and two clients through every fleet scenario.
    Fleet,
    /// `staging_discord`: every Discord scenario.
    Discord,
    /// `staging_load`: the sustained load run.
    Load,
}

impl StagingCheck {
    /// The check id: `staging_fleet`, `staging_discord` or `staging_load`.
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Fleet => "staging_fleet",
            Self::Discord => "staging_discord",
            Self::Load => "staging_load",
        }
    }

    /// The receipt file `<id>.<suffix>`.
    pub(crate) fn file(self, suffix: &str) -> String {
        format!("{}.{suffix}", self.id())
    }

    /// The fewest passing cases a passing receipt of this check reports.
    pub(crate) fn minimum_passing_cases(self) -> usize {
        match self {
            Self::Fleet => 50,
            Self::Discord => 13,
            Self::Load => 10,
        }
    }

    /// How long a run of this check may take, in seconds.
    pub(crate) fn time_limit_seconds(self) -> u64 {
        RUN_TIME_LIMIT_SECONDS
    }
}
