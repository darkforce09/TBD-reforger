//! Scale math.

use super::*;

/// Converts deck zoom to metres per screen pixel.
#[must_use]
pub fn m_per_px(deck_zoom: f64) -> f64 {
    if deck_zoom.is_finite() {
        2.0_f64.powf(-deck_zoom)
    } else {
        f64::NAN
    }
}

pub use website_map_engine::camera::grid_reference::GRID_STEP_M;

/// Width and height of the current terrain in metres.
pub const TERRAIN_SPAN_M: f64 = 12_800.0;

/// Maximum rendered width of the metric scale bar.
pub const SCALE_MAX_PX: f64 = 200.0;
/// Minimum rendered width of the metric scale bar.
pub const SCALE_MIN_PX: f64 = 120.0;

/// Distance, width, and label selected for a metric scale bar.
#[derive(Clone, Debug, PartialEq)]
pub struct ScaleBarSpec {
    pub dist_m: f64,
    pub width_px: f64,
    pub label: String,
}

/// Formats a map distance with an appropriate metric unit.
#[must_use]
pub fn format_distance(dist_m: f64) -> String {
    if dist_m >= 1000.0 {
        let km = dist_m / 1000.0;
        if (km.round() - km).abs() < 1e-9 {
            format!("{} km", km.round() as i64)
        } else {
            format!("{km:.1} km")
        }
    } else {
        format!("{} m", dist_m.round() as i64)
    }
}

/// Formats metres per pixel for the status readout.
#[must_use]
pub fn format_m_per_px(m_per_px: f64) -> String {
    if !m_per_px.is_finite() || m_per_px <= 0.0 {
        return "— m/px".to_string();
    }
    let decimals = decimals_for_mpp(m_per_px);
    let factor = 10f64.powi(decimals as i32);
    let rounded = (m_per_px * factor).round() / factor;
    let decimals = if rounded.is_finite() && rounded > 0.0 {
        decimals_for_mpp(rounded)
    } else {
        decimals
    };
    format!("{m_per_px:.decimals$} m/px")
}

fn decimals_for_mpp(m_per_px: f64) -> usize {
    if m_per_px >= 100.0 {
        0
    } else if m_per_px >= 10.0 {
        1
    } else if m_per_px >= 1.0 {
        2
    } else if m_per_px >= 0.1 {
        3
    } else if m_per_px >= 0.01 {
        4
    } else {
        5
    }
}

/// Chooses a round distance whose bar fits the target pixel range.
#[must_use]
pub fn pick_scale_bar(m_per_px: f64) -> ScaleBarSpec {
    const MANTISSA: [f64; 3] = [5.0, 2.0, 1.0];
    if !m_per_px.is_finite() || m_per_px <= 0.0 {
        return ScaleBarSpec {
            dist_m: 1.0,
            width_px: 1.0,
            label: format_distance(1.0),
        };
    }
    for exp in (0..=7).rev() {
        let decade = 10.0_f64.powi(exp);
        for m in MANTISSA {
            let dist = m * decade;
            let width = dist / m_per_px;
            if width <= SCALE_MAX_PX {
                return ScaleBarSpec {
                    dist_m: dist,
                    width_px: width,
                    label: format_distance(dist),
                };
            }
        }
    }
    ScaleBarSpec {
        dist_m: 1.0,
        width_px: 1.0 / m_per_px,
        label: format_distance(1.0),
    }
}
