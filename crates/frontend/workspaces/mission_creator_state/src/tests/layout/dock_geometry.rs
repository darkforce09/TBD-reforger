//! The dock geometry is an input contract: the width the browser lays a dock out at (parsed back
//! out of its mount class) and the inset the pointer unprojection uses are one number, carried
//! through to a world coordinate on the engine's own camera; the docks' bottom lands exactly on
//! the status bar's top edge.

use super::{
    DOCK_BOTTOM_PX, DOCK_LEFT_MOUNT, DOCK_LEFT_MOUNT_COLLAPSED, DOCK_RIGHT_MOUNT,
    DOCK_RIGHT_MOUNT_COLLAPSED, STATUSBAR_H_PX, dock_bottom_px, dock_left_px, dock_right_px,
    set_chrome_hidden, set_dock_left_collapsed, set_dock_right_collapsed, strip_top_px, tw_len_px,
    tw_width_px,
};
use camera_math::ortho::state::OrthoCamera;

/// Both collapse latches off and the chrome shown, so the accessors report the EXPANDED consts.
/// The latches are thread-locals shared with the dock collapse tests, which run on the same thread.
fn expanded() {
    set_dock_left_collapsed(false);
    set_dock_right_collapsed(false);
    set_chrome_hidden(false);
}

/// The width the browser lays
/// out (parsed back out of the mount class) and the width the pointer unprojection insets by (the
/// live accessor) are ONE number, and the proof is carried all the way through to a world
/// coordinate on the engine's own camera.
///
/// PERTURB / FAIL / RESTORE is inline and on the real failure mode: `w-64` is a 256 px class.
/// Feed it to the same camera and the pane's left edge lands 64 world metres
/// away — the assertion checks the drift is EXACTLY `Δpx / scale`, so the check has teeth in both
/// directions (a drift of zero would mean the unprojection ignores its x argument).
#[test]
fn the_mounted_dock_width_and_the_pointer_unprojection_are_one_number() {
    expanded();

    // (1) Class → px. The collapsed mounts deliberately state NO width: the wrapper shrinks to
    // the stub the dock renders, which is what makes the freed strip click-through to the map.
    let dom_left = tw_width_px(DOCK_LEFT_MOUNT).expect("the left mount must state a `w-*` width");
    let dom_right =
        tw_width_px(DOCK_RIGHT_MOUNT).expect("the right mount must state a `w-*` width");
    assert!(
        tw_width_px(DOCK_LEFT_MOUNT_COLLAPSED).is_none()
            && tw_width_px(DOCK_RIGHT_MOUNT_COLLAPSED).is_none(),
        "a collapsed dock's wrapper must state no width, or it keeps covering the map"
    );

    // (2) px → the const the input path insets by.
    assert!(
        (dom_left - dock_left_px()).abs() < f64::EPSILON,
        "the LEFT mount class lays out {dom_left} px but the pointer unprojection insets \
             by {} px — every click in the map pane would be off by the difference",
        dock_left_px()
    );
    assert!(
        (dom_right - dock_right_px()).abs() < f64::EPSILON,
        "the RIGHT mount class lays out {dom_right} px but the pointer unprojection \
             insets by {} px",
        dock_right_px()
    );

    // (3) The world-space consequence, on the engine's own camera (the exact type
    // `select_tool::frozen_camera` builds), full-bleed viewport like the editor's.
    let (w, h) = (1920.0, 1080.0);
    let (tx, ty, zoom) = (6400.0, 6400.0, -2.0);
    let cam = OrthoCamera::new(w, h, tx, ty, zoom);
    let scale = zoom.exp2();

    let edge_from_const = cam.unproject_xy(dock_left_px(), strip_top_px())[0];
    let edge_from_dom = cam.unproject_xy(dom_left, strip_top_px())[0];
    assert!(
        (edge_from_const - edge_from_dom).abs() < 1e-9,
        "the map pane's LEFT edge must be one world point whether you derive it from the \
             mount class or from the inset const"
    );
    let right_from_const = cam.unproject_xy(w - dock_right_px(), strip_top_px())[0];
    let right_from_dom = cam.unproject_xy(w - dom_right, strip_top_px())[0];
    assert!(
        (right_from_const - right_from_dom).abs() < 1e-9,
        "the map pane's RIGHT edge must be one world point from either derivation"
    );

    // (4) PERTURB — the earlier `w-64` class, left behind while the const moved to 240. This is the
    // half-edit the pin exists for, stated as a value so the check must reject it.
    let stale = tw_width_px("absolute bottom-0 left-0 top-12 z-20 w-64")
        .expect("the stale `w-64` class still parses");
    assert!(
        (stale - dom_left).abs() > f64::EPSILON,
        "PERTURB: the stale class must differ from the shipped one or this proves nothing"
    );
    let drifted = cam.unproject_xy(stale, strip_top_px())[0];
    let drift_m = (drifted - edge_from_const).abs();
    assert!(
        (drift_m - (stale - dom_left).abs() / scale).abs() < 1e-9,
        "PERTURB: an out-of-step class must shift the unprojected edge by exactly Δpx / scale \
             ({drift_m} m for {} px at scale {scale})",
        (stale - dom_left).abs()
    );
    assert!(
        drift_m > 1.0,
        "PERTURB: {} px of class/const drift is {drift_m} world metres of pointer error — \
             silent, plausible-looking, and invisible to every render test",
        (stale - dom_left).abs()
    );

    // RESTORE: the shipped pair is the zero-drift one.
    assert!((edge_from_const - edge_from_dom).abs() < 1e-9);
    expanded();
}

/// The docks stop AT the status bar's top edge, not over it. The bar is docked
/// `inset-x-0 bottom-0` at height [`STATUSBAR_H_PX`], so its top is `barY = h − STATUSBAR_H_PX`;
/// an expanded dock ends `bottom-9` = [`dock_bottom_px`] up, so its bottom is
/// `dockBottom = h − dock_bottom_px()`. The acceptance is `dockBottom <= barY` at every swept
/// viewport — and we hold it as EQUALITY (the dock's bottom lands exactly on the bar's top, no
/// gap, no overlap). This is the Y-axis twin of the class↔const width pin above: the mount's
/// `bottom-*` token, read back with [`tw_len_px`], must equal the accessor's number, or the DOM
/// and the acceptance-geometry model have drifted.
#[test]
fn the_docks_bottom_lands_on_the_status_bar_top() {
    expanded();

    // (1) class → px: both expanded mounts state `bottom-9`, and it resolves to DOCK_BOTTOM_PX
    // (= the bar height). The collapsed mounts deliberately state NO `bottom-*` (the wrapper
    // shrinks to its stub), exactly as they state no `w-*`.
    let dom_bottom_left =
        tw_len_px(DOCK_LEFT_MOUNT, "bottom-").expect("left mount must state a `bottom-*`");
    let dom_bottom_right =
        tw_len_px(DOCK_RIGHT_MOUNT, "bottom-").expect("right mount must state a `bottom-*`");
    assert!(
        (dom_bottom_left - DOCK_BOTTOM_PX).abs() < f64::EPSILON
            && (dom_bottom_right - DOCK_BOTTOM_PX).abs() < f64::EPSILON,
        "the mounts lay out bottom={dom_bottom_left}/{dom_bottom_right} px but \
             DOCK_BOTTOM_PX = {DOCK_BOTTOM_PX} — the DOM half drifted from the const"
    );
    assert_eq!(
        DOCK_BOTTOM_PX, STATUSBAR_H_PX,
        "the dock-bottom inset must equal the status bar's painted height, or the dock \
             edge cannot land on the bar's top edge"
    );
    assert!(
        tw_len_px(DOCK_LEFT_MOUNT_COLLAPSED, "bottom-").is_none()
            && tw_len_px(DOCK_RIGHT_MOUNT_COLLAPSED, "bottom-").is_none(),
        "a collapsed dock's wrapper must state no `bottom-*` (it shrinks to its stub)"
    );

    // (2) px → geometry: the review's acceptance, at every swept viewport. dockBottom <= barY.
    for (w, h) in [(1920.0, 1080.0), (1366.0, 768.0), (2560.0, 1440.0)] {
        let bar_y = h - STATUSBAR_H_PX; // bar docked bottom-0, height STATUSBAR_H_PX
        let dock_bottom = h - dock_bottom_px(); // expanded dock ends dock_bottom_px() up
        assert!(
            dock_bottom <= bar_y,
            "at {w}x{h} the dock bottom ({dock_bottom}) must be <= the bar top ({bar_y})"
        );
        assert!(
            (dock_bottom - bar_y).abs() < f64::EPSILON,
            "at {w}x{h} the dock bottom ({dock_bottom}) should land exactly on the bar \
                 top ({bar_y}) — no overlap (the O-1 defect) and no dead gap"
        );
    }

    // (3) PERTURB — the earlier class ran `bottom-0`, i.e. a 0 px inset. Stated as a value the
    // check must reject: it puts the dock bottom at the viewport floor, BELOW the bar top by
    // exactly STATUSBAR_H_PX — the overlap that let the containers eat the bar's clicks.
    let stale_bottom = tw_len_px("absolute bottom-0 left-0 top-12 z-20 w-60", "bottom-")
        .expect("the stale `bottom-0` class still parses");
    assert!(
        (stale_bottom - DOCK_BOTTOM_PX).abs() > f64::EPSILON,
        "PERTURB: the stale `bottom-0` inset must differ from the shipped one or this proves \
             nothing"
    );
    let h = 1080.0;
    let bar_y = h - STATUSBAR_H_PX;
    let stale_dock_bottom = h - stale_bottom; // = 1080, the viewport floor
    assert!(
        stale_dock_bottom > bar_y,
        "PERTURB: `bottom-0` puts the dock bottom at {stale_dock_bottom}, BELOW the bar top \
             ({bar_y}) — a {STATUSBAR_H_PX} px overlap that swallows every click aimed at the bar"
    );

    // RESTORE.
    expanded();
}
