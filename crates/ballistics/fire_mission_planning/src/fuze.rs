//! Time-fuze setting for a burst above the target: of one charge, or of the lowest charge
//! that fuzes.
//!
//! **Role:** solves every charge of the shell onto the burst point, the target raised by the
//! burst height, and reports the time of flight to that point as the fuze time, either for one
//! given charge ([`solve_time_fuze`]) or for the lowest charge whose burst aim solves with a
//! time inside the fuze window ([`solve_time_fuze_over_charges`]); a refused setting names
//! why. The shell's default fuze time is reported with every setting.
//!
//! **Position:** the `fire_mission_planning` crate; the fire-mission assembler
//! ([`crate::fire_mission`]) calls these for a time-fuzed shell
//! (illumination, smoke) of a
//! [`ballistics_model::catalog::BallisticsCatalog`]. The burst point is
//! solved by [`ballistics_solver::solve_fire_solution`], so the
//! bracket, the bounded iterations, the high-angle branch, the refusals and the
//! wind-corrected aim are the solver's own charge selection.
//!
//! **Signals & state:** none; pure functions over plain values and a borrowed catalog.
//!
//! **Invariants:**
//! - The fuze time is the time to the descending point at `(D, Δh + burst_height_m)`, `D` the
//!   gun-to-target distance and `Δh` the target height minus the gun height; the aim that
//!   produces it is returned as [`TimeFuzeSolution::burst_aim`], whose `rings` is the charge.
//! - The charge search picks the fewest rings whose time lies in `[min_s, max_s]`; no charge
//!   with fewer rings has a settable time.
//! - A time outside `[min_s, max_s]` at every charge considered that reaches the burst point
//!   is [`FuzeRefusal::OutsideFuzeWindow`]; when no charge considered reaches it, the refusal
//!   is the solver's cause at the lowest such charge ([`FuzeRefusal::of_burst_point`]: above
//!   the apex, beyond the maximum range, inside the minimum range, no convergence, beyond the
//!   shell's lifetime, invalid input); a refused setting carries no time, so a refused value
//!   can never be set on a fuze.
//! - A shell without a time fuze or charges, an unknown charge, a NaN or infinite burst height
//!   or a malformed fuze window is a [`FuzeError`]; no input panics.
//! - [`FuzeSetting`] projects the contract's `FuzeSetting` without its `burst_aim`, which
//!   [`TimeFuzeSolution::burst_aim`] carries beside it; the full projection is
//!   [`crate::fire_mission::FireMissionFuze`].
//!
//! @contract fire-mission.schema.json#/definitions/FuzeSetting partial
//! @contract fire-mission.schema.json#/definitions/FuzeRefusal

use serde::{Deserialize, Serialize};
use thiserror::Error;

use ballistics_model::catalog::{BallisticsCatalog, CatalogLookupError, TimeFuze};
use ballistics_model::ids::ShellId;
use ballistics_solver::{
    ChargeSolution, FireSolution, FireSolutionError, FireSolutionRequest, MapPosition,
    SolutionRefusal, solve_fire_solution,
};

/// Why a fuze setting is refused: the burst point is reached only outside the fuze window, or
/// the solver's cause for not reaching it at the charge the setting describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FuzeRefusal {
    /// Every allowed charge that reaches the burst point does so at a time outside the
    /// shell's fuze window.
    OutsideFuzeWindow,
    /// The burst point lies above the apex of the flight at the maximum elevation.
    AboveApex,
    /// The burst point lies beyond the maximum range.
    BeyondRange,
    /// The burst point lies inside the minimum range the maximum elevation sets.
    InsideMinimumRange,
    /// The elevation root search or the wind-corrected aim loop ran out of iterations.
    DidNotConverge,
    /// No elevation that reaches the burst point gets there within the shell's lifetime.
    TimeToLiveExceeded,
    /// A NaN, infinite or non-physical burst-point input.
    InvalidInput,
}

impl FuzeRefusal {
    /// The refusal of a burst point the solver refuses with `refusal`.
    pub fn of_burst_point(refusal: SolutionRefusal) -> Self {
        match refusal {
            SolutionRefusal::Unreachable => Self::AboveApex,
            SolutionRefusal::OutOfRange => Self::BeyondRange,
            SolutionRefusal::TooClose => Self::InsideMinimumRange,
            SolutionRefusal::DidNotConverge => Self::DidNotConverge,
            SolutionRefusal::TimeToLiveExceeded => Self::TimeToLiveExceeded,
            SolutionRefusal::InvalidInput => Self::InvalidInput,
        }
    }

    /// Whether the burst point is reached, so only the fuze window refuses the setting.
    pub fn reaches_burst_point(self) -> bool {
        self == Self::OutsideFuzeWindow
    }
}

/// The fuze time for a burst above the target, or the refusal.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FuzeSetting {
    /// Burst height above the target, metres.
    pub burst_height_m: f64,
    /// Fuze time in seconds; `None` when refused.
    pub time_s: Option<f64>,
    /// Shortest settable fuze time, seconds.
    pub min_s: f64,
    /// Longest settable fuze time, seconds.
    pub max_s: f64,
    /// The shell's default fuze time, seconds.
    pub default_s: f64,
    /// Why the setting is refused; `None` when it is not.
    pub refusal: Option<FuzeRefusal>,
}

/// A fuze setting with the charge row of the burst-point solve that produced it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeFuzeSolution {
    /// The setting.
    pub setting: FuzeSetting,
    /// The charge solved onto the burst point: its aim azimuth, elevation and time of flight,
    /// or its refusal.
    pub burst_aim: ChargeSolution,
}

/// Why no fuze setting can be computed.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum FuzeError {
    /// The catalog has no such weapon, shell, pairing or charge.
    #[error(transparent)]
    Lookup(#[from] CatalogLookupError),
    /// The shell carries no time fuze.
    #[error("shell `{shell_id}` has no time fuze")]
    ShellHasNoTimeFuze {
        /// Identifier of the shell.
        shell_id: ShellId,
    },
    /// The shell has no charge to fire.
    #[error("shell `{shell_id}` has no charge")]
    ShellHasNoCharges {
        /// Identifier of the shell.
        shell_id: ShellId,
    },
    /// The burst height or a fuze window value is NaN, infinite or out of its domain.
    #[error("fuze input `{parameter}` is invalid (got {value})")]
    InvalidInput {
        /// Name of the offending input.
        parameter: &'static str,
        /// The value that was refused.
        value: f64,
    },
    /// The burst-point request has no firing solution at all.
    #[error(transparent)]
    FireSolution(#[from] FireSolutionError),
}

/// The fuze setting of the charge of `rings` rings for a burst `burst_height_m` above the
/// target of `request`.
///
/// # Errors
///
/// [`FuzeError::InvalidInput`] for a non-finite burst height,
/// [`FuzeError::Lookup`] for an unknown weapon, shell or charge,
/// [`FuzeError::ShellHasNoTimeFuze`], [`FuzeError::InvalidInput`] for a malformed window, and
/// [`FuzeError::FireSolution`] for a request-wide solver fault.
pub fn solve_time_fuze(
    catalog: &BallisticsCatalog,
    request: &FireSolutionRequest<'_>,
    rings: u32,
    burst_height_m: f64,
) -> Result<TimeFuzeSolution, FuzeError> {
    require("burst_height_m", burst_height_m, burst_height_m.is_finite())?;
    catalog.resolve_firing(request.weapon_id, request.shell_id, rings)?;
    let (fuze, solution) = solve_burst_point(catalog, request, burst_height_m)?;
    let burst_aim = solution
        .charges
        .iter()
        .find(|charge| charge.rings == rings)
        .copied()
        .ok_or_else(|| CatalogLookupError::UnknownRing {
            shell_id: request.shell_id.to_owned(),
            rings,
        })?;
    Ok(TimeFuzeSolution {
        setting: fuze_setting(&fuze, burst_height_m, burst_time_of_flight(&burst_aim)),
        burst_aim,
    })
}

/// The fuze setting of the lowest charge of the shell whose burst-point aim solves with a
/// time inside the fuze window, for a burst `burst_height_m` above the target of `request`.
///
/// Every charge is solved onto the burst point by the solver's own charge selection. When no
/// charge fuzes, the refusal names why: [`FuzeRefusal::OutsideFuzeWindow`] with the lowest
/// charge that reaches the burst point when one does, else the lowest charge with its solver
/// refusal as [`FuzeRefusal::of_burst_point`] names it.
///
/// # Errors
///
/// [`FuzeError::InvalidInput`] for a non-finite burst height or a malformed window,
/// [`FuzeError::Lookup`] for an unknown weapon or shell, [`FuzeError::ShellHasNoTimeFuze`],
/// [`FuzeError::ShellHasNoCharges`], and [`FuzeError::FireSolution`] for a request-wide
/// solver fault.
pub fn solve_time_fuze_over_charges(
    catalog: &BallisticsCatalog,
    request: &FireSolutionRequest<'_>,
    burst_height_m: f64,
) -> Result<TimeFuzeSolution, FuzeError> {
    let (fuze, solution) = solve_burst_point(catalog, request, burst_height_m)?;
    let lowest = |accepts: &dyn Fn(Result<f64, SolutionRefusal>) -> bool| {
        solution
            .charges
            .iter()
            .filter(|row| accepts(burst_time_of_flight(row)))
            .min_by_key(|row| row.rings)
            .copied()
    };
    let burst_aim = lowest(&|time| time.is_ok_and(|time_s| within_window(&fuze, time_s)))
        .or_else(|| lowest(&|time| time.is_ok()))
        .or_else(|| lowest(&|_| true))
        .ok_or_else(|| FuzeError::ShellHasNoCharges {
            shell_id: request.shell_id.to_owned(),
        })?;
    Ok(TimeFuzeSolution {
        setting: fuze_setting(&fuze, burst_height_m, burst_time_of_flight(&burst_aim)),
        burst_aim,
    })
}

/// The setting of `fuze` for a burst `burst_height_m` above the target reached after
/// `time_of_flight` seconds, or the solver's refusal of the burst point.
pub fn fuze_setting(
    fuze: &TimeFuze,
    burst_height_m: f64,
    time_of_flight: Result<f64, SolutionRefusal>,
) -> FuzeSetting {
    let (time_s, refusal) = match time_of_flight {
        Ok(time_s) if within_window(fuze, time_s) => (Some(time_s), None),
        Ok(_) => (None, Some(FuzeRefusal::OutsideFuzeWindow)),
        Err(refusal) => (None, Some(FuzeRefusal::of_burst_point(refusal))),
    };
    FuzeSetting {
        burst_height_m,
        time_s,
        min_s: fuze.min_s,
        max_s: fuze.max_s,
        default_s: fuze.default_s,
        refusal,
    }
}

/// Every charge of the shell solved onto the target of `request` raised by
/// `burst_height_m`, with the shell's validated fuze window.
fn solve_burst_point(
    catalog: &BallisticsCatalog,
    request: &FireSolutionRequest<'_>,
    burst_height_m: f64,
) -> Result<(TimeFuze, FireSolution), FuzeError> {
    require("burst_height_m", burst_height_m, burst_height_m.is_finite())?;
    let (_, shell) = catalog.weapon_and_shell(request.weapon_id, request.shell_id)?;
    let fuze = shell
        .time_fuze
        .ok_or_else(|| FuzeError::ShellHasNoTimeFuze {
            shell_id: request.shell_id.to_owned(),
        })?;
    validate_window(&fuze)?;
    let burst_request = FireSolutionRequest {
        target: MapPosition {
            height_m: request.target.height_m + burst_height_m,
            ..request.target
        },
        ..*request
    };
    Ok((fuze, solve_fire_solution(catalog, &burst_request)?))
}

/// The time of flight of a burst-point row, or its refusal.
fn burst_time_of_flight(row: &ChargeSolution) -> Result<f64, SolutionRefusal> {
    match (row.time_of_flight_s, row.refusal) {
        (Some(time_s), None) => Ok(time_s),
        (_, refusal) => Err(refusal.unwrap_or(SolutionRefusal::InvalidInput)),
    }
}

/// Whether `time_s` is a settable fuze time of `fuze`, both ends included.
fn within_window(fuze: &TimeFuze, time_s: f64) -> bool {
    time_s >= fuze.min_s && time_s <= fuze.max_s
}

/// Checks `0 ≤ min_s ≤ default_s ≤ max_s`, all finite.
fn validate_window(fuze: &TimeFuze) -> Result<(), FuzeError> {
    require(
        "min_s",
        fuze.min_s,
        fuze.min_s.is_finite() && fuze.min_s >= 0.0,
    )?;
    require(
        "max_s",
        fuze.max_s,
        fuze.max_s.is_finite() && fuze.max_s >= fuze.min_s,
    )?;
    require(
        "default_s",
        fuze.default_s,
        fuze.default_s.is_finite() && fuze.default_s >= fuze.min_s && fuze.default_s <= fuze.max_s,
    )
}

fn require(parameter: &'static str, value: f64, valid: bool) -> Result<(), FuzeError> {
    if valid {
        Ok(())
    } else {
        Err(FuzeError::InvalidInput { parameter, value })
    }
}

#[cfg(test)]
#[path = "tests/fuze.rs"]
mod tests;
