//! Why a firing problem is refused as a whole.
//!
//! **Role:** the crate's one error type and its `Result` alias, gathering the request-wide error
//! families the solver reports: a fire solution's, a crest clearance's and a dispersion's.
//! **Position:** converted into with `?` from the crate's fallible calls, so a caller that solves,
//! checks clearance and computes dispersion in one function returns one error type. A per-charge
//! [`crate::SolutionRefusal`] is a value of a solved row, not an error of the request.
//! **Signals & state:** none; plain data.
//! **Invariants:** each variant wraps its family's error transparently: the message and the
//! source are the wrapped error's own.

use crate::crest_clearance::CrestClearanceError;
use crate::dispersion::DispersionError;
use crate::fire_solution::FireSolutionError;

/// Why a fire solution, a crest clearance or a dispersion is refused.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The request names an unknown weapon or shell, or carries an invalid value.
    #[error(transparent)]
    FireSolution(#[from] FireSolutionError),
    /// The terrain profile or the solved charge cannot be checked for clearance.
    #[error(transparent)]
    CrestClearance(#[from] CrestClearanceError),
    /// The solved charge's dispersion cannot be computed.
    #[error(transparent)]
    Dispersion(#[from] DispersionError),
}

/// The result of a fallible call of this crate; the error defaults to [`Error`].
pub type Result<T, E = Error> = core::result::Result<T, E>;
