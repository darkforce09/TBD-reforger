//! The transform gizmo's vertical Z arm: its geometry, its hit test and its elevation arithmetic.
//!
//! **Role:** holds the four pure numbers an elevation drag is made of — where the arm's shaft
//! sits relative to the gizmo centre, whether a press landed on that shaft, what a vertical
//! cursor travel is worth in metres of height, and how that height quantises and reads out.
//! **Position:** the leaf of the canvas belt. `canvas/overlays.rs` draws the arm from
//! [`Z_ARM_LENGTH`] and turns a drag into snapped metres through [`dy_to_elevation`] and
//! [`snap_elevation`]; `canvas/gestures.rs` calls [`hit_z_arm`] on pointerdown to decide that a
//! press owns Z rather than XY, and [`format_height_readout`] to label the drag on screen.
//! Nothing here touches Leptos, the DOM or the map engine.
//! **Signals & state:** none. Every function is pure over its arguments.
//! **Invariants:** arm geometry is in canvas CSS pixels measured from the gizmo centre with
//! screen Y growing downward, so the arm occupies the same on-screen rectangle at every zoom
//! level. Heights are metres, and an upward drag raises. The shaft deliberately excludes the
//! band around the centre, which belongs to the entity's XY grab.

/// How far the Z arm's shaft reaches above the gizmo centre, in canvas CSS pixels.
pub const Z_ARM_LENGTH: f64 = 60.0;
/// How wide the Z arm's grab band is, in canvas CSS pixels, centred on the gizmo's vertical axis.
///
/// It doubles as the height of the dead band directly above the centre that [`hit_z_arm`] refuses,
/// so the arm and the entity's XY grab never claim the same pixel.
pub const Z_ARM_WIDTH: f64 = 14.0;

/// True when a press at `press_x` / `press_y` landed on the Z arm of the gizmo centred at
/// `center_x` / `center_y`, all in canvas CSS pixels.
///
/// The claimed region is the exposed vertical shaft only: within half [`Z_ARM_WIDTH`] of the
/// centre's vertical axis, and above the centre by more than [`Z_ARM_WIDTH`] but no more than
/// [`Z_ARM_LENGTH`]. Excluding the band next to the centre is what keeps an already-selected
/// entity's second XY drag working — the centre is the XY grab, and an arm that reached it would
/// steal every drag that starts there.
///
/// `_scale` is unread: the arm is drawn in screen pixels and so is hit-tested in screen pixels,
/// and it stays in the signature because the gesture layer hands every canvas hit test the camera
/// scale.
pub fn hit_z_arm(press_x: f64, press_y: f64, center_x: f64, center_y: f64, _scale: f64) -> bool {
    let dx = (press_x - center_x).abs();
    let on_x = dx <= Z_ARM_WIDTH / 2.0;
    // The entity/XY grab occupies the center. Only the exposed vertical shaft owns Z;
    // including the center steals the second XY drag of an already selected entity.
    let on_y = press_y < center_y - Z_ARM_WIDTH && press_y >= center_y - Z_ARM_LENGTH;
    on_x && on_y
}

/// Vertical cursor travel `dy` in canvas CSS pixels → metres of elevation change, at a camera
/// `scale` of pixels per metre.
///
/// The sign flip is the whole point: screen Y grows downward and +Z is up, so dragging the arm
/// upward must raise the selection. The caller owns guarding `scale` — a zero or non-finite
/// divisor here yields a non-finite height.
pub fn dy_to_elevation(dy: f64, scale: f64) -> f64 {
    -dy / scale
}

/// Round an elevation `z` in metres onto a ladder of `step`-metre rungs.
///
/// A `step` of `0.0` or less means "no snap" and passes `z` through unchanged, so the call sites
/// that read a snap ladder which may be switched off need no special case of their own.
pub fn snap_elevation(z: f64, step: f64) -> f64 {
    if step > 0.0 {
        (z / step).round() * step
    } else {
        z
    }
}

/// An elevation in metres as the chip beside the arm shows it: one decimal place and a `m` unit.
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
        assert!(!hit_z_arm(100.0, 100.0, 100.0, 100.0, 1.0)); // XY center
        assert!(!hit_z_arm(100.0, 86.0, 100.0, 100.0, 1.0)); // center boundary
        assert!(hit_z_arm(100.0, 85.0, 100.0, 100.0, 1.0)); // exposed shaft
    }

    #[test]
    fn test_dy_to_elevation() {
        let elev = dy_to_elevation(-10.0, 2.0);
        assert_eq!(elev, 5.0, "dy to elevation must go RED if inverted");
    }
}
