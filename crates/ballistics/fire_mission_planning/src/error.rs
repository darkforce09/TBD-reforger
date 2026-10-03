//! Why a fire mission, a battery or a fuze has no answer.
//!
//! **Role:** the crate's one error type and its `Result` alias, gathering the error families the
//! planner reports: a fire mission's refusal, a battery's error and a fuze's error.
//! **Position:** converted into with `?` from the crate's fallible calls, so a caller that
//! assembles a mission and sets fuzes in one function returns one error type. A per-charge
//! [`crate::fuze::FuzeRefusal`] is a value of a solved fuze row, not an error.
//! **Signals & state:** none; plain data.
//! **Invariants:** each variant wraps its family's error transparently: the message and the
//! source are the wrapped error's own.

use crate::battery::BatteryError;
use crate::fire_mission::FireMissionRefusal;
use crate::fuze::FuzeError;

/// Why a fire mission, a battery or a fuze setting is refused.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The fire mission has no solution at all.
    #[error(transparent)]
    FireMission(#[from] FireMissionRefusal),
    /// The battery has no solution.
    #[error(transparent)]
    Battery(#[from] BatteryError),
    /// No fuze setting can be computed.
    #[error(transparent)]
    Fuze(#[from] FuzeError),
}

/// The result of a fallible call of this crate; the error defaults to [`Error`].
pub type Result<T, E = Error> = core::result::Result<T, E>;
