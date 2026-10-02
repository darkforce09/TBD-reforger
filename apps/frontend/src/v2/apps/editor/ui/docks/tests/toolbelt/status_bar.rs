use crate::v2::core::test_support::class_r_scrub::{live_code, live_source};

/// This module's file, with comments blanked but string literals KEPT — so the Tailwind class
/// strings and the readout labels survive as structural landmarks for ordering proofs.
fn src_kept() -> String {
    live_source(super::test_source::raw_toolbelt())
}

/// (structure) The single conflated pill is split into TWO components — a tools mount and a
/// readouts mount — which is what makes them two independent mount points in `mission_editor`.
#[test]
fn tools_and_readouts_are_two_separate_components() {
    let src = live_code(super::test_source::raw_toolbelt());
    let mode_fn = format!("pub fn {}", "ModeToolbar(");
    let status_fn = format!("pub fn {}", "StatusBar(");
    assert!(
        src.contains(&mode_fn) && src.contains(&status_fn),
        "T-636: the belt must split into a ModeToolbar AND a StatusBar component"
    );
}

/// (no conflation) The tools mount carries ONLY tools — none of the CUR/OBJ/SEL/SZ readout
/// labels leak into `ModeToolbar`; they all live in `StatusBar`. Proven by slicing each
/// component body out of the string-kept source and checking where the labels land.
#[test]
fn mode_toolbar_holds_no_readouts_and_status_bar_holds_them() {
    let src = src_kept();
    let mode_at = src
        .find(&format!("fn {}", "ModeToolbar("))
        .expect("ModeToolbar present");
    let status_at = src
        .find(&format!("fn {}", "StatusBar("))
        .expect("StatusBar present");
    assert!(
        mode_at < status_at,
        "ModeToolbar must be defined before StatusBar"
    );
    let mode_body = &src[mode_at..status_at];
    let status_at2 = status_at;
    let compat_at = src
        .find(&format!("fn {}", "BottomToolbelt("))
        .expect("compat shim present");
    let status_body = &src[status_at2..compat_at];

    // The three tool controls live in the toolbar (Select active + Ruler/LoS stubs).
    for tool in ["Select", "Ruler", "LoS"] {
        assert!(
            mode_body.contains(tool),
            "ModeToolbar must carry the {tool} tool"
        );
    }
    // The readout labels must NOT be in the toolbar…
    for label in ["\"CUR\"", "\"OBJ\"", "\"SEL\"", "\"SZ\""] {
        // (labels appear as `"OBJ"` etc. in the view; `Cursor` titles are separate.)
        let bare = label.trim_matches('"');
        let quoted = format!("\"{bare}\"");
        assert!(
            !mode_body.contains(&quoted),
            "T-636: readout {bare} must not live in the tools mount (that conflation is the bug)"
        );
        // …and every readout label must be in the status bar.
        assert!(
            status_body.contains(&quoted),
            "T-636: readout {bare} must live in the full-width StatusBar"
        );
    }
    // The status bar spans the viewport (full width), not a centred fixed pill: its surface
    // recipe carries `w-full`, and the component wears that recipe.
    let recipe_at = src
        .find(&format!("const {}", "STATUSBAR"))
        .expect("STATUSBAR recipe present");
    let recipe = &src[recipe_at
        ..src[recipe_at..]
            .find(';')
            .map(|i| recipe_at + i)
            .unwrap_or(src.len())];
    assert!(
        recipe.contains("w-full"),
        "T-636: the STATUSBAR recipe must be full-width (w-full), stretched across the viewport"
    );
    assert!(
        status_body.contains("class=STATUSBAR"),
        "T-636: StatusBar must wear the full-width STATUSBAR recipe"
    );
}

/// T-787 — [`STATUSBAR_H_PX`] is the SOURCE OF TRUTH for the bar's top edge, so it must equal
/// the `h-*` token actually painted in [`STATUSBAR`]. `eden_layout::dock_bottom_px` insets the
/// docks by exactly this number so `dock.bottom == bar.y`; if someone re-heights the bar (say
/// `h-9` → `h-10`) without bumping the const, the docks would resume overlapping the bar and the
/// O-1 click-eating defect would return — this pin fails loudly instead.
#[test]
fn statusbar_height_const_tracks_the_h_token() {
    let painted = crate::v2::apps::editor::shell::layout::tw_len_px(super::STATUSBAR, "h-")
        .expect("the STATUSBAR recipe must state an `h-*` height");
    assert!(
        (painted - super::STATUSBAR_H_PX).abs() < f64::EPSILON,
        "T-787: STATUSBAR paints h-{} px but STATUSBAR_H_PX = {} — the dock-bottom inset is \
         derived from the const, so a drift re-opens the O-1 overlap (docks eat bar clicks)",
        painted,
        super::STATUSBAR_H_PX
    );
}

/// (T-667) The map-furniture slot is FILLED with the scale bar, and what fills it is pinned
/// here rather than left to a reader's assumption.
///
/// The slot keeps its `flex-1` spacer (it still owns the bar's middle and pushes the HUD + OPEN
/// to the right), gains `justify-center` so the bar sits in the CLEAR CENTRE SPAN the wave-105
/// verifier said clears both docks (left 256 / right 320 occluded until T-721), and now contains
/// a `ScaleBar` child. The edge grid references are NOT here — they anchor to the map-pane edges
/// and render from `MapGridRefs` (pinned separately below).
#[test]
fn fills_the_t667_furniture_slot_with_the_scale_bar() {
    let src = src_kept();
    let hook = format!("data-status-{}", "furniture");
    let at = src
        .find(&hook)
        .expect("T-667: the map-furniture slot must exist");
    // Still carries the flex spacer that owns the middle, AND centres its content in the clear
    // span between the docks.
    let open_end = at + src[at..].find('>').expect("furniture div opens");
    let open_tag = &src[at..=open_end];
    assert!(
        open_tag.contains("flex-1"),
        "T-667: the furniture slot must keep the flex-1 spacer (it owns the bar's middle)"
    );
    assert!(
        open_tag.contains("justify-center"),
        "T-667: the furniture slot must centre its content (the clear centre span the docks miss)"
    );
    // The slot is now FILLED: its body contains the ScaleBar child (built this ticket), not the
    // empty div the guard used to pin.
    let body = src[open_end + 1..]
        .split_once("</div>")
        .map(|(b, _)| b)
        .unwrap_or("");
    assert!(
        body.contains("ScaleBar"),
        "T-667: the furniture slot must now render the ScaleBar (the guard's do-not-build-early \
         state is deliberately retired — the slot is filled, not empty)"
    );
}

/// (T-667) The scale bar and the edge grid references are both real components with the maths the
/// ticket names. Proven on scrubbed code (strings blanked) so a needle can't hide in a class
/// string or comment: `ScaleBar` picks the round distance from the zoom, `MapGridRefs` renders
/// the pane-edge references, and both reuse the CUR/heartbeat channel (cursor + debug_hud) rather
/// than a new rAF loop.
#[test]
fn t667_components_and_reactivity_channel() {
    let code = live_code(super::test_source::raw_toolbelt());
    // Both public components exist.
    assert!(
        code.contains(&format!("pub fn {}", "ScaleBar("))
            && code.contains(&format!("pub fn {}", "MapGridRefs(")),
        "T-667: ScaleBar + MapGridRefs must be real components"
    );
    // Scale bar drives its width off the zoom→m/px picker, not a hardcoded size.
    assert!(
        code.contains("pick_scale_bar(") && code.contains("m_per_px("),
        "T-667: the scale bar must size from pick_scale_bar(m_per_px(zoom))"
    );
    // Grid refs anchor to the MAP-PANE insets read by NAME from eden_layout (not viewport edges),
    // and project lines through the shared camera.
    assert!(
        code.contains("DOCK_LEFT_PX")
            && code.contains("DOCK_RIGHT_PX")
            && code.contains("STRIP_TOP_PX")
            && code.contains("edge_eastings(")
            && code.contains("edge_northings("),
        "T-667: grid refs must anchor to the map-pane insets and use the edge-label geometry"
    );
    // Reactivity reuses the existing channel: the components read the live camera off the
    // registered engine and re-run on the cursor (pan) + debug_hud (~1 Hz zoom) heartbeats.
    // No `request_animation_frame` is introduced in this file.
    assert!(
        code.contains("camera_snapshot()"),
        "T-667: the components must read the live camera via world_assets::camera_snapshot"
    );
    assert!(
        !code.contains("request_animation_frame"),
        "T-667: reuse the CUR/heartbeat channel — do NOT add a new rAF loop in eden_toolbelt"
    );
}

/// (T-793 / O-2) The grid-ref `<For>` rows are keyed by SCREEN POSITION, never the label text.
/// This is the render half of the O-2 fix and cannot be reached by the native property test (the
/// `<For>` is a Leptos view innard), so it is pinned on scrubbed code: the `MapGridRefs` body must
/// key on `l.key` and must NOT key on `l.text`. A text key is retained across a pan, so Leptos
/// reuses the node and freezes its `left:` (the hostile review's half-updated set); the position
/// key busts on every move. Proven on `live_code` (strings blanked) so the needle is the real
/// `key=` binding, not a mention in a comment or class string.
#[test]
fn grid_ref_for_is_keyed_by_position_not_text() {
    let code =
        crate::v2::core::test_support::class_r_scrub::live_code(super::test_source::raw_toolbelt());
    let body = crate::v2::core::test_support::class_r_scrub::only_body(
        &code,
        &format!("pub fn {}", "MapGridRefs("),
    );
    // The rows key on the position-derived identity…
    assert!(
        body.contains("key=|l| l.key"),
        "T-793: MapGridRefs rows must be keyed by the position key (l.key), not the label text — \
         a text key retains a moved node and freezes its left: (O-2)"
    );
    // …and never on the text (the reverted defect). Both `<For>`s (eastings + northings) count.
    assert!(
        !body.contains("key=|l| l.text"),
        "T-793: a grid-ref `<For>` keyed on l.text is the O-2 defect — Leptos would reuse the \
         node for a retained ref and hold its stale screen x across a pan"
    );
    let key_bindings = body.matches("key=|l| l.key").count();
    assert!(
        key_bindings >= 2,
        "T-793: both the easting and northing `<For>` must key by position (found {key_bindings})"
    );
}

/// (F-13, wave-201 addendum) The Eden-style coordinate unit: a real cursor/selection axis reads
/// `<n> m` (Eden `X 8762.61 m`, frame 170422), the off-map cell stays the bare em-dash, and the
/// numeric PRECISION is byte-for-byte `fmt_coord` (three decimals, correct to the metre) — the
/// suffix is presentation, no value is lost, so the CUR readout stays the trusted oracle the
/// grid-label acceptance test unprojects against.
#[test]
fn eden_coord_readout_carries_the_metre_unit() {
    use super::{fmt_coord, fmt_coord_eden};
    // A real value gains ` m` and keeps `fmt_coord`'s exact digits.
    let e = fmt_coord_eden(Some(8762.61));
    assert!(
        e.ends_with(" m"),
        "F-13: a coordinate axis must read Eden's `<n> m`, got `{e}`"
    );
    assert_eq!(
        e.trim_end_matches(" m"),
        fmt_coord(Some(8762.61)),
        "F-13: the number must be EXACTLY fmt_coord's — the ` m` is presentation, not a reformat \
         (no precision loss: the CUR readout stays the metre-accurate oracle)"
    );
    assert!(
        e.contains("8762.610"),
        "F-13: three-decimal precision is preserved under the unit, got `{e}`"
    );
    // The off-map "no value" cell is the bare em-dash — a unit on nothing would be a lie.
    assert_eq!(
        fmt_coord_eden(None),
        fmt_coord(None),
        "F-13: the off-map cell must stay the em-dash, with NO ` m` suffix"
    );
    assert!(
        !fmt_coord_eden(None).contains('m'),
        "F-13: no unit on the absent readout"
    );
}

/// (F-13) The StatusBar's axis assembly RENDERS the unit-suffixed formatter, not the bare
/// `fmt_coord` — proven on the string-kept live source (the addendum's `live_source` pin), so a
/// future edit that drops the unit back to the un-suffixed readout fails here. The X/Y/Z cells
/// go through `fmt_coord_eden`.
#[test]
fn status_bar_axis_readout_uses_the_eden_unit_formatter() {
    let src = live_source(super::test_source::raw_toolbelt());
    let body = crate::v2::core::test_support::class_r_scrub::only_body(
        &src,
        &format!("pub fn {}", "StatusBar("),
    );
    assert!(
        body.contains("fmt_coord_eden("),
        "F-13: the StatusBar axis readout must render the Eden ` m`-suffixed value via \
         fmt_coord_eden — the coordinate cells carry the unit (Eden `X … m`)"
    );
}

/// (§Open) The primary-action slot exists on its own surface at the right end — Eden's
/// `PLAY SCENARIO` position; ours is OPEN. The slot is built; the button's behaviour is the
/// undecided part of §Open, so it is inert here.
#[test]
fn builds_the_open_primary_action_slot() {
    let src = src_kept();
    let hook = format!("data-status-{}", "open");
    assert!(
        src.contains(&hook),
        "§Open: the primary-action slot (OPEN) must be built at the bar's right end"
    );
    let label = ["OP", "EN"].concat();
    let at = src.find(&hook).expect("open slot present");
    let window = &src[at..src[at..]
        .find("</button>")
        .map(|i| at + i)
        .unwrap_or(src.len())];
    assert!(
        window.contains(&label) && window.contains("folder_open"),
        "§Open: the slot must present an OPEN button (label + folder_open glyph)"
    );
}

/// (T-719) The debug HUD gets a legitimate VISIBLE home inside the status bar's right section,
/// BEFORE the OPEN slot, gated on `hud_shown` (Ctrl+Alt+D) AND a non-empty sampler string. The
/// `chrome_hidden` half of the gate is the StatusBar mount wrapper (pinned in `mission_editor`).
#[test]
fn hud_slot_is_gated_and_sits_before_open() {
    // Gate expression on scrubbed code (strings blanked) so it is the real gate, not a comment.
    let code = live_code(super::test_source::raw_toolbelt());
    assert!(
        code.contains("on && !text.is_empty()"),
        "T-719: the HUD slot must render only when (hud_shown AND non-empty sampler string)"
    );
    // hud_shown / debug_hud are real optional props threaded into StatusBar.
    assert!(
        code.contains("hud_shown") && code.contains("debug_hud"),
        "T-719: StatusBar must accept the HUD toggle + text signals"
    );
    // Ordering: the HUD slot precedes the OPEN slot in the right section.
    let src = src_kept();
    let hud = src
        .find(&format!("data-status-{}", "hud"))
        .expect("HUD slot present");
    let open = src
        .find(&format!("data-status-{}", "open"))
        .expect("OPEN slot present");
    assert!(
        hud < open,
        "T-719: the HUD slot must sit BEFORE the OPEN slot"
    );
}
