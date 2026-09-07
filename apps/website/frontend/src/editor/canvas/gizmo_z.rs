pub const Z_ARM_LENGTH: f64 = 60.0;
pub const Z_ARM_WIDTH: f64 = 14.0;

pub fn hit_z_arm(press_x: f64, press_y: f64, center_x: f64, center_y: f64, _scale: f64) -> bool {
    let dx = (press_x - center_x).abs();
    let on_x = dx <= Z_ARM_WIDTH / 2.0;
    let on_y = press_y <= center_y && press_y >= center_y - Z_ARM_LENGTH;
    on_x && on_y
}

pub fn dy_to_elevation(dy: f64, scale: f64) -> f64 {
    -dy / scale
}

pub fn snap_elevation(z: f64, step: f64) -> f64 {
    if step > 0.0 {
        (z / step).round() * step
    } else {
        z
    }
}

pub fn format_height_readout(z: f64) -> String {
    format!("{z:.1} m")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hit_z_arm() {
        assert!(
            hit_z_arm(100.0, 50.0, 100.0, 100.0, 1.0),
            "Z arm above gizmo center returns None or fails"
        );
        assert!(!hit_z_arm(100.0, 110.0, 100.0, 100.0, 1.0)); // below center
        assert!(!hit_z_arm(120.0, 50.0, 100.0, 100.0, 1.0)); // too far right
    }

    #[test]
    fn test_dy_to_elevation() {
        let elev = dy_to_elevation(-10.0, 2.0);
        assert_eq!(elev, 5.0, "dy to elevation must go RED if inverted");
    }
}
