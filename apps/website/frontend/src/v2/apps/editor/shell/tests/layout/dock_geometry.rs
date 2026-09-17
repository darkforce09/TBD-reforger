//! Layout t637 dock geometry tests.

//! T-637 — **THE DOCK GEOMETRY IS AN INPUT CONTRACT, NOT A STYLESHEET.**
//!
//! Equalising the docks to Eden's 240/240 is a two-line change to a pair of `f64`s and a pair of
//! Tailwind classes — and getting those two halves out of step is the most dangerous edit in this
//! file, because it fails SILENTLY. `select_tool` unprojects the pointer by the `f64`s; the browser
//! lays the panels out from the classes. If the class says 256 and the const says 240, every panel
//! still draws correctly, the map still draws correctly, and every click inside the map pane resolves
//! to a world position 16 px wrong — 64 world metres at a typical zoom. No screenshot shows it and no
//! render test catches it.
//!
//! These pins close that loop end to end: the mount class parses back to the const ([`tw_width_px`]),
//! the const is what the live accessor reports, and the accessor is what a real
//! `map_engine_core::camera::OrthoCamera` unprojects with. The perturbation fires the rule on the
//! exact half-edit it exists to catch — the pre-T-637 `w-64` class left behind while the const moved.

use super::{
    dock_bottom_px, dock_left_px, dock_right_px, set_chrome_hidden, set_dock_left_collapsed,
    set_dock_right_collapsed, strip_top_px, tw_len_px, tw_width_px, BTN_ICON, DOCK_BOTTOM_PX,
    DOCK_L, DOCK_LEFT_MOUNT, DOCK_LEFT_MOUNT_COLLAPSED, DOCK_LEFT_PX, DOCK_PX, DOCK_R,
    DOCK_RIGHT_MOUNT, DOCK_RIGHT_MOUNT_COLLAPSED, DOCK_RIGHT_PX, ROW_MENUS, ROW_MENUS_PX,
    ROW_TOOLS, ROW_TOOLS_PX, STRIP_ROWS, STRIP_TOP_PX,
};
use crate::v2::apps::editor::ui::docks::toolbelt::STATUSBAR_H_PX;
use crate::v2::core::test_support::class_r_scrub::live_code;
use website_map_engine::camera::ortho::state::OrthoCamera;

/// Both collapse latches off and the chrome shown, so the accessors report the EXPANDED consts.
/// The latches are thread-locals shared with `t638_collapse`, which runs on the same thread.
fn expanded() {
    set_dock_left_collapsed(false);
    set_dock_right_collapsed(false);
    set_chrome_hidden(false);
}

/// **THE EQUALISATION.** Eden is 240/240 in every one of the 75 screenshots; we were 256 left and
/// 320 right. The asymmetry was not cosmetic — a 320 px right dock is what pushed its trailing tab
/// off the viewport (the T-632 clipping this ticket absorbed), and an off-centre map pane is what
/// made the collapse reflow feel like a jump.
///
/// Stated structurally: both sides resolve to the ONE [`DOCK_PX`], so "equal" is a definition
/// rather than two literals that happen to match today.
#[test]
fn the_docks_are_one_equalised_width() {
    assert_eq!(
        DOCK_LEFT_PX, DOCK_RIGHT_PX,
        "T-637: Eden's docks are the same width; ours were 256/320"
    );
    assert_eq!(DOCK_LEFT_PX, DOCK_PX);
    assert_eq!(DOCK_RIGHT_PX, DOCK_PX);
    assert!(
        (DOCK_PX - 240.0).abs() < f64::EPSILON,
        "T-637: the equalised width is Eden's 240, got {DOCK_PX}"
    );
    // The dock widths and the STRIP height are DIFFERENT contracts — equalising X must not have
    // moved Y. T-634's two-row split depends on this number.
    assert!(
        (STRIP_TOP_PX - 48.0).abs() < f64::EPSILON,
        "T-637 must not touch the strip height contract"
    );
}

/// **THE SILENT-FAILURE PIN, AND THE ONE THIS TICKET MOST NEEDED.** The width the browser lays
/// out (parsed back out of the mount class) and the width the pointer unprojection insets by (the
/// live accessor) are ONE number, and the proof is carried all the way through to a world
/// coordinate on the engine's own camera.
///
/// PERTURB / FAIL / RESTORE is inline and on the real failure mode: `w-64` is the class this
/// ticket replaced. Feed it to the same camera and the pane's left edge lands 64 world metres
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
        "T-638: a collapsed dock's wrapper must state no width, or it keeps covering the map"
    );

    // (2) px → the const the input path insets by.
    assert!(
        (dom_left - dock_left_px()).abs() < f64::EPSILON,
        "T-637: the LEFT mount class lays out {dom_left} px but the pointer unprojection insets \
             by {} px — every click in the map pane would be off by the difference",
        dock_left_px()
    );
    assert!(
        (dom_right - dock_right_px()).abs() < f64::EPSILON,
        "T-637: the RIGHT mount class lays out {dom_right} px but the pointer unprojection \
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
        "T-637: the map pane's LEFT edge must be one world point whether you derive it from the \
             mount class or from the inset const"
    );
    let right_from_const = cam.unproject_xy(w - dock_right_px(), strip_top_px())[0];
    let right_from_dom = cam.unproject_xy(w - dom_right, strip_top_px())[0];
    assert!(
        (right_from_const - right_from_dom).abs() < 1e-9,
        "T-637: the map pane's RIGHT edge must be one world point from either derivation"
    );

    // (4) PERTURB — the pre-T-637 class, left behind while the const moved to 240. This is the
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

/// T-787 (O-1) — the docks stop AT the status bar's top edge, not over it. The bar is docked
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
        "T-787: the mounts lay out bottom={dom_bottom_left}/{dom_bottom_right} px but \
             DOCK_BOTTOM_PX = {DOCK_BOTTOM_PX} — the DOM half drifted from the const"
    );
    assert_eq!(
        DOCK_BOTTOM_PX, STATUSBAR_H_PX,
        "T-787: the dock-bottom inset must equal the status bar's painted height, or the dock \
             edge cannot land on the bar's top edge"
    );
    assert!(
        tw_len_px(DOCK_LEFT_MOUNT_COLLAPSED, "bottom-").is_none()
            && tw_len_px(DOCK_RIGHT_MOUNT_COLLAPSED, "bottom-").is_none(),
        "T-787: a collapsed dock's wrapper must state no `bottom-*` (it shrinks to its stub)"
    );

    // (2) px → geometry: the review's acceptance, at every swept viewport. dockBottom <= barY.
    for (w, h) in [(1920.0, 1080.0), (1366.0, 768.0), (2560.0, 1440.0)] {
        let bar_y = h - STATUSBAR_H_PX; // bar docked bottom-0, height STATUSBAR_H_PX
        let dock_bottom = h - dock_bottom_px(); // expanded dock ends dock_bottom_px() up
        assert!(
            dock_bottom <= bar_y,
            "T-787: at {w}x{h} the dock bottom ({dock_bottom}) must be <= the bar top ({bar_y})"
        );
        assert!(
            (dock_bottom - bar_y).abs() < f64::EPSILON,
            "T-787: at {w}x{h} the dock bottom ({dock_bottom}) should land exactly on the bar \
                 top ({bar_y}) — no overlap (the O-1 defect) and no dead gap"
        );
    }

    // (3) PERTURB — the pre-T-787 class ran `bottom-0`, i.e. a 0 px inset. Stated as a value the
    // check must reject: it puts the dock bottom at the viewport floor, BELOW the bar top by
    // exactly STATUSBAR_H_PX — the overlap that let the containers eat the bar's clicks.
    let stale_bottom = tw_len_px("absolute bottom-0 left-0 top-12 z-20 w-60", "bottom-")
        .expect("the stale `bottom-0` class still parses");
    assert!(
        (stale_bottom - DOCK_BOTTOM_PX).abs() > f64::EPSILON,
        "PERTURB: the stale `bottom-0` inset must differ from the shipped one or this proves \
             nothing"
    );
    let (w, h) = (1920.0, 1080.0);
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

/// The mount classes are RENDERED, not merely declared: `mission_editor` names these consts
/// instead of hand-writing a second copy of the width. Checked on scrubbed code (strings and
/// comments blanked), sliced from the page fn so the file's leading `#[cfg(test)]` helper does
/// not cut the body away — the t636/t662 idiom.
///
/// Exactly one use each: a second mount would be a second place the width could drift.
#[test]
fn mission_editor_mounts_the_docks_from_these_consts() {
    let raw = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/mission_editor.rs"
    ));
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let editor = live_code(&raw[raw.find(anchor.as_str()).expect("anchor present")..]);
    for name in [
        "editor::shell::layout::DOCK_LEFT_MOUNT",
        "editor::shell::layout::DOCK_LEFT_MOUNT_COLLAPSED",
        "editor::shell::layout::DOCK_RIGHT_MOUNT",
        "editor::shell::layout::DOCK_RIGHT_MOUNT_COLLAPSED",
    ] {
        assert!(
            editor.contains(name),
            "T-637: `{name}` must be RENDERED by mission_editor — a hand-written class literal \
                 is a second source for a width the pointer unprojection has to agree with"
        );
    }
    // Exactly four `eden_layout::DOCK_*` reads in the page body: two per dock (expanded +
    // collapsed). A fifth is a second mount, i.e. a second place the geometry can drift.
    assert_eq!(
        editor.matches("editor::shell::layout::DOCK_").count(),
        4,
        "T-637: the docks mount in exactly two places, each reading its expanded/collapsed pair"
    );
}

/// T-637 — the T-634 FOLD-BACK. `STRIP_ROWS` / `ROW_MENUS` / `ROW_TOOLS` / `ROW_*_PX` now live
/// here beside their siblings, and the dead one-row `STRIP` they replaced is deleted.
///
/// `STRIP`'s load-bearing claim survives its deletion: the strip is made of the SAME glass as the
/// docks it sits above (that shared surface is why the chrome reads as one product rather than
/// as assembled parts). It used to be checked by comparing the two shells to each other; it is
/// now checked directly against the docks, which is what it always meant.
#[test]
fn the_strip_shell_folded_back_and_still_shares_the_docks_glass() {
    let surface = "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl";
    for (name, recipe) in [
        ("STRIP_ROWS", STRIP_ROWS),
        ("DOCK_L", DOCK_L),
        ("DOCK_R", DOCK_R),
    ] {
        assert!(
            recipe.starts_with(surface),
            "T-637: `{name}` must open with the shared docked-overlay glass"
        );
    }
    // The height contract T-634 built the split for, restated where the consts now live.
    assert!(
        (ROW_MENUS_PX + ROW_TOOLS_PX - STRIP_TOP_PX).abs() < f64::EPSILON,
        "T-637: the two strip rows SPLIT {STRIP_TOP_PX} px, they do not add to it"
    );
    assert!(
        ROW_MENUS.contains("h-6"),
        "the menu row is the FIXED half of the split"
    );
    assert!(
        ROW_TOOLS.contains("flex-1") && !ROW_TOOLS.contains("h-["),
        "the tool row takes the REMAINDER — two stated heights could drift from STRIP_TOP_PX"
    );
    // The dead shell is gone. Needle assembled so this test's own source cannot satisfy it.
    let layout = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/shell/layout.rs"
    ));
    let dead = format!("{} STRIP:", "pub(crate) const");
    assert!(
            !layout.contains(&dead),
            "T-637: the one-row `STRIP` shell had no consumer left after T-634 and is deleted — \
             `#![allow(dead_code)]` means nothing warns, so this is the only thing that would notice \
             it coming back"
        );
    // The equalised docks are a flex COLUMN each, so a dock body can claim the leftover height
    // instead of leaving it as void below a short tree.
    for (name, dock) in [("DOCK_L", DOCK_L), ("DOCK_R", DOCK_R)] {
        assert!(
            dock.contains("flex") && dock.contains("flex-col") && dock.contains("h-full"),
            "T-637: `{name}` must be a full-height column — the void under the tree was a dock \
                 that never told its children they could grow"
        );
    }
}

/// T-637 — **the tripwire T-634 left, resolved.** `BTN_ICON` used to rest at
/// `text-on-surface-variant` with `p-1.5`: a live glyph looked like a dead one, and a 36 px
/// control could not sit in a 24 px strip row. T-634 could not fix it (this file was another
/// slice's owns that wave) and made a local `TOOL_ICON` copy instead, which left the defect
/// standing for every OTHER caller — the help panel's close button, the docks, the toolbelt.
///
/// The recipe is now fixed at the source and the copy is deleted, so the fix reaches every
/// caller. The pin is INVERTED from T-634's, deliberately: its premise was that `BTN_ICON` is
/// still muted, and that premise is exactly what this ticket had to falsify.
#[test]
fn btn_icon_rests_bright_and_fits_a_dense_row() {
    assert!(
        BTN_ICON.contains("text-on-surface") && !BTN_ICON.contains("text-on-surface-variant"),
        "T-637: a LIVE icon button rests at full strength; dimming is reserved for DISABLED, \
             where it means something"
    );
    assert!(
        BTN_ICON.contains("p-0.5") && !BTN_ICON.contains("p-1.5"),
        "T-637: `p-1.5` around a 24 px line box is a 36 px control — too tall for a 24 px strip \
             row or a dense dock"
    );
    assert!(
        !BTN_ICON.contains("hover:") && !BTN_ICON.contains("disabled:"),
        "T-637: state comes from the T-668 vocabulary at the call site (HOVER_FILL / \
             DISABLED_GLYPH), not from an ad-hoc pair baked into the geometry recipe"
    );
    // The local copy is gone from the strip. Needle assembled so this source cannot satisfy it.
    let strip = [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/ui/docks/top_strip.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/ui/docks/top_strip/arrange.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/ui/docks/top_strip/menu_catalog.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/ui/docks/top_strip/clock_and_draft.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/ui/docks/top_strip/row_mirror.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/ui/docks/top_strip/dialog_focus.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/ui/docks/top_strip/mission_summary.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/ui/docks/top_strip/view.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/ui/docks/top_strip/view/menu_row.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/ui/docks/top_strip/view/tool_row.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/ui/docks/top_strip/view/overlays.rs"
        )),
    ]
    .concat();
    let copy = format!("{} TOOL_ICON", "const");
    assert!(
        !strip.contains(&copy),
        "T-637: `TOOL_ICON` existed only to route around the muted BTN_ICON; with the recipe \
             fixed it is a second source of truth for the same geometry"
    );
}
