//! The uniform Catmull-Rom spline through four control points, with its plan-view direction.
//!
//! **Role:** [`evaluate_uniform_catmull_rom`] gives the point at a parameter on the segment
//! between the middle two control points, the unit tangent of the curve projected on the XZ plane,
//! and the normal that turns that tangent a quarter turn ([`SplineSample`]).
//! **Position:** the water export image lane samples each river segment with it and stamps a
//! cross-section along the normal.
//! **Signals & state:** none; a pure function and a plain `Copy` value.
//! **Invariants:** points are `[x, y, z]` with y up; the tangent and normal have a zero y and unit
//! length in the XZ plane, falling back to an unnormalised division by 1 where the plan tangent
//! vanishes or is NaN; every polynomial is evaluated in one fixed order with no fused multiply-add,
//! so a sample is the same to the bit on every target.

/// One sample of the spline: the point, and the plan-view tangent and normal there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplineSample {
    /// The point on the curve, `[x, y, z]`.
    pub position: [f64; 3],
    /// The curve's direction in the XZ plane, `[tx, 0, tz]`, of unit length unless the plan
    /// derivative is zero or NaN.
    pub tangent: [f64; 3],
    /// The tangent turned a quarter turn in the XZ plane, `[-tz, 0, tx]`.
    pub normal: [f64; 3],
}

/// The cubic coefficients of one axis: `a·t³ + b·t² + v0·t + p1`.
struct AxisCubic {
    cubic: f64,
    quadratic: f64,
    start_velocity: f64,
    start: f64,
}

impl AxisCubic {
    /// The coefficients of one axis from its four control values, with the end velocities half
    /// the chord across each end's neighbours.
    fn new(p0: f64, p1: f64, p2: f64, p3: f64) -> Self {
        let start_velocity = (p2 - p0) * 0.5;
        let end_velocity = (p3 - p1) * 0.5;
        Self {
            cubic: 2.0 * p1 - 2.0 * p2 + start_velocity + end_velocity,
            quadratic: -3.0 * p1 + 3.0 * p2 - 2.0 * start_velocity - end_velocity,
            start_velocity,
            start: p1,
        }
    }

    /// The axis value at `t`, given `t²` and `t³`.
    fn value(&self, t: f64, t_squared: f64, t_cubed: f64) -> f64 {
        self.cubic * t_cubed + self.quadratic * t_squared + self.start_velocity * t + self.start
    }

    /// The axis derivative at `t`, given `t²`.
    fn derivative(&self, t: f64, t_squared: f64) -> f64 {
        3.0 * self.cubic * t_squared + 2.0 * self.quadratic * t + self.start_velocity
    }
}

/// The uniform Catmull-Rom sample at `t` (0 at `p1`, 1 at `p2`) of the curve through `p0`, `p1`,
/// `p2` and `p3`; see [`SplineSample`] for the direction it reports.
#[must_use]
pub fn evaluate_uniform_catmull_rom(
    p0: [f64; 3],
    p1: [f64; 3],
    p2: [f64; 3],
    p3: [f64; 3],
    t: f64,
) -> SplineSample {
    let t_squared = t * t;
    let t_cubed = t_squared * t;
    let x = AxisCubic::new(p0[0], p1[0], p2[0], p3[0]);
    let y = AxisCubic::new(p0[1], p1[1], p2[1], p3[1]);
    let z = AxisCubic::new(p0[2], p1[2], p2[2], p3[2]);
    let tangent_x = x.derivative(t, t_squared);
    let tangent_z = z.derivative(t, t_squared);
    let plan_length = tangent_x.hypot(tangent_z);
    // A zero or NaN length divides by one instead, leaving the raw derivative.
    let length = if plan_length == 0.0 || plan_length.is_nan() {
        1.0
    } else {
        plan_length
    };
    SplineSample {
        position: [
            x.value(t, t_squared, t_cubed),
            y.value(t, t_squared, t_cubed),
            z.value(t, t_squared, t_cubed),
        ],
        tangent: [tangent_x / length, 0.0, tangent_z / length],
        normal: [-tangent_z / length, 0.0, tangent_x / length],
    }
}

#[cfg(test)]
#[path = "tests/catmull_rom_tests.rs"]
mod tests;
