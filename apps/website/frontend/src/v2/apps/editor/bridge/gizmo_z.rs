//! Elevation drag geometry and readouts.
/// Length of the elevation gizmo arm in screen pixels.
pub const Z_ARM_LENGTH: f64 = 60.0;
/// Hit width of the elevation gizmo arm in screen pixels.
pub const Z_ARM_WIDTH: f64 = 14.0;

/// Tests whether a screen press lands on the elevation arm.
pub fn hit_z_arm(press_x: f64, press_y: f64, center_x: f64, center_y: f64, _scale: f64) -> bool {
    let dx = (press_x - center_x).abs();
    let on_x = dx <= Z_ARM_WIDTH / 2.0;
    let on_y = press_y < center_y - Z_ARM_WIDTH && press_y >= center_y - Z_ARM_LENGTH;
    on_x && on_y
}

/// Converts vertical pointer motion to an elevation change.
pub fn dy_to_elevation(dy: f64, scale: f64) -> f64 {
    -dy / scale
}

/// Rounds an elevation to the active height increment.
pub fn snap_elevation(z: f64, step: f64) -> f64 {
    if step > 0.0 {
        (z / step).round() * step
    } else {
        z
    }
}

/// Formats an elevation in metres for the drag readout.
pub fn format_height_readout(z: f64) -> String {
    format!("{z:.1} m")
}

#[cfg(test)]
#[path = "tests/gizmo_z/hit_and_elevation.rs"]
mod tests;
