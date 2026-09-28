//! One-dimensional searches over the elevation: the maximum-range elevation and the elevation
//! whose range equals the target distance.
//!
//! **Role:** golden-section maximisation of a range function over an elevation bracket, and a
//! bracketed root search (Brent's method with a bisection fallback) with a hard iteration cap.
//!
//! **Position:** `ballistics/solver`;
//! [`crate::data::scenario::ballistics::solver::charge_selection`] hands both searches a closure
//! that flies the shell at one elevation. The searches know no flight model, shell or unit
//! beyond "a real number in, a real number out", so any caller with a bracketed scalar problem
//! can use them.
//!
//! **Signals & state:** none; each call owns its bracket and evaluates the caller's closure.
//!
//! **Invariants:**
//! - [`maximise_by_golden_section`] evaluates the objective exactly `2 + iterations` times and
//!   answers the best point it evaluated; a non-finite objective value never wins, and a tie
//!   moves the bracket towards the upper end.
//! - [`find_bracketed_root`] needs finite end values of opposite sign (or a zero), keeps a sign
//!   change inside its bracket at every step, stops once the bracket is at most
//!   [`RootSearchLimits::tolerance`] wide, and never evaluates the function more than
//!   [`RootSearchLimits::max_iterations`] times after the two ends; running out of iterations is
//!   [`RootSearchError::DidNotConverge`], never a guess.
//! - Only `+ - × /` and comparisons are used, so native and WASM builds produce the same bits.

use thiserror::Error;

/// Golden-section iterations of the maximum-range search.
pub const GOLDEN_SECTION_ITERATIONS: usize = 40;

/// `(√5 - 1) / 2`, the golden-section interior ratio.
const INVERSE_GOLDEN_RATIO: f64 = 0.618_033_988_749_894_8;

/// How long a bracketed root search may run and how narrow its bracket must get.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RootSearchLimits {
    /// The most function evaluations after the two bracket ends.
    pub max_iterations: usize,
    /// The bracket width at which the search stops, in the argument's unit.
    pub tolerance: f64,
}

impl RootSearchLimits {
    /// The limits of the elevation root search: 60 iterations, a bracket of 1e-9 radians.
    pub const ELEVATION: RootSearchLimits = RootSearchLimits {
        max_iterations: 60,
        tolerance: 1e-9,
    };
}

/// Why a bracketed root search found no root.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
pub enum RootSearchError<E> {
    /// The end values are NaN, infinite or of the same non-zero sign.
    #[error("the bracket [{lower}, {upper}] holds no sign change")]
    NoSignChange {
        /// Lower bracket end.
        lower: f64,
        /// Upper bracket end.
        upper: f64,
    },
    /// The function answered NaN or an infinity inside the bracket.
    #[error("the function is not finite at {argument}")]
    NonFiniteValue {
        /// The argument whose value is not finite.
        argument: f64,
    },
    /// The bracket was still wider than the tolerance after the last allowed iteration.
    #[error("no convergence after {iterations} iterations (bracket {bracket_width} wide)")]
    DidNotConverge {
        /// Iterations spent.
        iterations: usize,
        /// Bracket width when the search stopped.
        bracket_width: f64,
    },
    /// The caller's function refused an argument.
    #[error("the function refused an argument")]
    Evaluation(E),
}

/// The best point a golden-section search evaluated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Maximum {
    /// The argument of the largest value.
    pub argument: f64,
    /// The largest value; `f64::NEG_INFINITY` when no evaluation was finite.
    pub value: f64,
}

/// Maximises `objective` over `[lower, upper]` by golden-section search.
///
/// The objective answers `f64::NEG_INFINITY` (or any non-finite value) where it has no value;
/// such points never become the maximum. On a tie the bracket moves towards `upper`, so a
/// region without values at the lower end is left behind.
pub fn maximise_by_golden_section(
    mut objective: impl FnMut(f64) -> f64,
    lower: f64,
    upper: f64,
    iterations: usize,
) -> Maximum {
    let mut best = Maximum {
        argument: upper,
        value: f64::NEG_INFINITY,
    };
    let mut evaluate = |argument: f64| {
        let raw = objective(argument);
        let value = if raw.is_finite() {
            raw
        } else {
            f64::NEG_INFINITY
        };
        if value > best.value {
            best = Maximum { argument, value };
        }
        value
    };
    let (mut low, mut high) = (lower, upper);
    let mut left = high - INVERSE_GOLDEN_RATIO * (high - low);
    let mut right = low + INVERSE_GOLDEN_RATIO * (high - low);
    let mut left_value = evaluate(left);
    let mut right_value = evaluate(right);
    for _ in 0..iterations {
        if left_value <= right_value {
            low = left;
            left = right;
            left_value = right_value;
            right = low + INVERSE_GOLDEN_RATIO * (high - low);
            right_value = evaluate(right);
        } else {
            high = right;
            right = left;
            right_value = left_value;
            left = high - INVERSE_GOLDEN_RATIO * (high - low);
            left_value = evaluate(left);
        }
    }
    best
}

/// Finds a root of `function` inside `[lower, upper]`, given its values there.
///
/// Brent's method: inverse quadratic interpolation or the secant step when it lands well inside
/// the bracket and shrinks fast enough, bisection otherwise.
///
/// # Errors
///
/// [`RootSearchError::NoSignChange`] for end values that are not finite or share a sign,
/// [`RootSearchError::NonFiniteValue`] for a non-finite interior value,
/// [`RootSearchError::Evaluation`] when `function` refuses an argument, and
/// [`RootSearchError::DidNotConverge`] when the iterations run out.
pub fn find_bracketed_root<E>(
    mut function: impl FnMut(f64) -> Result<f64, E>,
    (lower, lower_value): (f64, f64),
    (upper, upper_value): (f64, f64),
    limits: RootSearchLimits,
) -> Result<f64, RootSearchError<E>> {
    let no_sign_change = RootSearchError::NoSignChange { lower, upper };
    if !lower_value.is_finite() || !upper_value.is_finite() {
        return Err(no_sign_change);
    }
    if lower_value == 0.0 {
        return Ok(lower);
    }
    if upper_value == 0.0 {
        return Ok(upper);
    }
    if (lower_value > 0.0) == (upper_value > 0.0) {
        return Err(no_sign_change);
    }

    // `best` holds the best estimate, `contra` the other side of the sign change, `previous`
    // the estimate before `best`.
    let (mut previous, mut previous_value) = (lower, lower_value);
    let (mut best, mut best_value) = (upper, upper_value);
    let (mut contra, mut contra_value) = (previous, previous_value);
    let mut step = best - previous;
    let mut step_before = step;
    let half_tolerance = 0.5 * limits.tolerance;
    for iteration in 0..=limits.max_iterations {
        if (best_value > 0.0) == (contra_value > 0.0) {
            contra = previous;
            contra_value = previous_value;
            step = best - previous;
            step_before = step;
        }
        if contra_value.abs() < best_value.abs() {
            previous = best;
            previous_value = best_value;
            best = contra;
            best_value = contra_value;
            contra = previous;
            contra_value = previous_value;
        }
        let midpoint_offset = 0.5 * (contra - best);
        if midpoint_offset.abs() <= half_tolerance || best_value == 0.0 {
            return Ok(best);
        }
        if iteration == limits.max_iterations {
            return Err(RootSearchError::DidNotConverge {
                iterations: iteration,
                bracket_width: (contra - best).abs(),
            });
        }
        if step_before.abs() >= half_tolerance && previous_value.abs() > best_value.abs() {
            let (numerator, denominator) = interpolation_step(
                (previous, previous_value),
                (best, best_value),
                (contra, contra_value),
                midpoint_offset,
            );
            let bound = (3.0 * midpoint_offset * denominator
                - (half_tolerance * denominator).abs())
            .min((step_before * denominator).abs());
            if 2.0 * numerator < bound {
                step_before = step;
                step = numerator / denominator;
            } else {
                step = midpoint_offset;
                step_before = step;
            }
        } else {
            step = midpoint_offset;
            step_before = step;
        }
        previous = best;
        previous_value = best_value;
        best += if step.abs() > half_tolerance {
            step
        } else {
            half_tolerance.copysign(midpoint_offset)
        };
        best_value = function(best).map_err(RootSearchError::Evaluation)?;
        if !best_value.is_finite() {
            return Err(RootSearchError::NonFiniteValue { argument: best });
        }
    }
    unreachable!("the loop returns on its last iteration")
}

/// The interpolated step `numerator / denominator` from `best`, with a non-negative numerator:
/// the secant step when only two distinct points exist, inverse quadratic interpolation
/// otherwise.
fn interpolation_step(
    (previous, previous_value): (f64, f64),
    (best, best_value): (f64, f64),
    (contra, contra_value): (f64, f64),
    midpoint_offset: f64,
) -> (f64, f64) {
    let s = best_value / previous_value;
    let (mut numerator, mut denominator) = if previous == contra {
        (2.0 * midpoint_offset * s, 1.0 - s)
    } else {
        let q = previous_value / contra_value;
        let r = best_value / contra_value;
        (
            s * (2.0 * midpoint_offset * q * (q - r) - (best - previous) * (r - 1.0)),
            (q - 1.0) * (r - 1.0) * (s - 1.0),
        )
    };
    if numerator > 0.0 {
        denominator = -denominator;
    } else {
        numerator = -numerator;
    }
    (numerator, denominator)
}
