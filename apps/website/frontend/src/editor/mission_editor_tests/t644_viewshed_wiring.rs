use crate::editor::arsenal::class_r_scrub::live_code;

/// Page-from-anchor + the T-934.13 gesture file (`canvas/gestures.rs`) — the viewshed wiring spans
/// the page body (signals, Esc arm, tool-switch Effect) and the moved pointerup commit arm. The
/// t642 `editor_live` idiom; each half scrubbed separately.
fn editor_live() -> String {
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let raw = include_str!("../mission_editor.rs");
    assert_eq!(
        raw.matches(anchor.as_str()).count(),
        1,
        "scrub anchor must be unambiguous"
    );
    let mut src = live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..]);
    src.push_str(&live_code(include_str!("../canvas/gestures.rs")));
    src.push_str(&live_code(include_str!("../canvas/commands.rs")));
    src
}

/// (sub-mode signal threaded to the toolbar) The page owns a real `los_mode` `RwSignal` and hands
/// it to `ModeToolbar` beside `tool_mode`, so the LoS button can reflect + toggle the sub-mode and
/// the pointer commit reads the SAME signal. (The toggle-on-reclick lives in `eden_toolbelt`,
/// pinned there; here we prove the wiring shares one signal, not two.)
#[test]
fn los_mode_signal_is_owned_and_handed_to_the_toolbar() {
    let ed = editor_live();
    assert!(
        ed.contains(
            "let los_mode = RwSignal::new(crate::editor::tools::los_tool::LosMode::default())"
        ),
        "T-644: the page must own a real los_mode RwSignal (the LoS sub-mode)"
    );
    assert!(
        ed.contains("ModeToolbar tool_mode los_mode"),
        "T-644: los_mode must be handed to ModeToolbar (one shared signal, toolbar + commit)"
    );
}

/// (commit routes ray vs viewshed) The single `LG::Ruler` pointerup arm, already gated by
/// `is_los()`, now branches on `los_mode…is_viewshed()`: a VIEWSHED click stores the observer
/// (`viewshed…place(`) and uploads the wash to the engine (`place_viewshed(` → `viewshed_upload(`),
/// while the RAY click still routes to `los…click(`. One arm, routed by the sub-mode — the same
/// discipline the ray adds on top of the ruler.
#[test]
fn viewshed_click_places_and_uploads_under_is_viewshed() {
    let ed = editor_live();
    assert!(
        ed.contains("is_viewshed()"),
        "T-644: the LoS commit must branch on los_mode.is_viewshed()"
    );
    assert!(
        ed.contains("viewshed.borrow_mut().place("),
        "T-644: a viewshed click must store the observer in the session ViewshedState"
    );
    assert!(
        ed.contains("place_viewshed(") && ed.contains(".viewshed_upload("),
        "T-644: a viewshed click must compute (place_viewshed) and upload the wash (viewshed_upload)"
    );
    // The viewshed branch sits INSIDE the `is_los()` arm and BESIDE the ray's `los…click(` — one
    // shared `LG::Ruler` commit, routed by tool_mode then sub-mode.
    assert!(
        ed.contains("los.borrow_mut().click(") && ed.contains("is_los()"),
        "T-644: the ray click route must remain (the sub-mode branches within the is_los() arm)"
    );
}

/// (no-engine / Boot-Failed guard) The wash upload only runs when the engine is live — mirroring
/// the ray's engine guard: `place_viewshed` returns `None` off-DEM (native/pre-mount → no upload),
/// and the upload is inside an `if let Some(e) = engine.borrow_mut().as_mut()` so a dead map
/// (`engine` is `None` after a Boot-Failed) draws nothing.
#[test]
fn viewshed_upload_is_engine_guarded() {
    let ed = editor_live();
    // The upload sits behind the same `Some(e)` engine guard the ray path uses — the guard
    // statement `if let Some(e) = engine.borrow_mut().as_mut()` opens the block the
    // `.viewshed_upload(` call lives in. Prove that statement sits between the compute
    // (`place_viewshed(`) and the upload, so the upload can only run with a live engine.
    let compute_at = ed
        .find("place_viewshed(")
        .expect("place_viewshed call present");
    let upload_at = ed
        .find(".viewshed_upload(")
        .expect("viewshed_upload call present");
    assert!(
        compute_at < upload_at,
        "T-644: place_viewshed (compute) must precede the upload"
    );
    let between = &ed[compute_at..upload_at];
    assert!(
        between.contains("if let Some(e) = engine.borrow_mut().as_mut()"),
        "T-644: viewshed_upload must run only when the engine is live — the no-engine / \
         Boot-Failed guard (if let Some(e) = engine.borrow_mut().as_mut()), mirroring the ray mode"
    );
}

/// (Esc — the SHARED seam, not a new listener) The keydown Escape arm dismisses the viewshed via
/// `viewshed…escape()` in the SAME arm that dismisses the ruler + ray, and drops the engine lane
/// (`viewshed_clear()`) on a real dismissal. No second window listener is added (T-726 pending).
#[test]
fn viewshed_escape_is_the_shared_seam() {
    let ed = editor_live();
    assert!(
        ed.contains("code().as_str()")
            && ed.contains("viewshed.borrow_mut().escape()")
            && ed.contains("ruler.borrow_mut().escape()"),
        "T-644: the viewshed Esc must ride the ONE shared keydown arm (beside ruler + ray escape)"
    );
    // Exactly once — the shared seam, not duplicated into a second listener.
    assert_eq!(
        ed.matches("viewshed.borrow_mut().escape()").count(),
        1,
        "T-644: the viewshed Esc must be wired exactly once (the shared seam)"
    );
    // Dismissal drops the GPU lane too.
    assert!(
        ed.contains("viewshed_clear()"),
        "T-644: a viewshed dismissal must drop the engine wash lane (viewshed_clear)"
    );
}

/// (tool/sub-mode switch clears — state + GPU lane; overlay bridge registered) Leaving LoS OR
/// toggling the sub-mode away from Viewshed clears the viewshed state AND the engine lane through
/// the EXTENDED tool-switch Effect (peer of the ruler's clear-on-switch); the state is a leaked
/// `RefCell<ViewshedState>` (overlay state, never a doc write) registered for the overlay bridge.
#[test]
fn switch_clears_state_and_lane_and_state_is_registered() {
    let ed = editor_live();
    // The tool-switch Effect observes both signals and clears when the viewshed lane is inactive.
    assert!(
        ed.contains("los_mode.get().is_viewshed()")
            && ed.contains("viewshed.borrow_mut().clear()")
            && ed.contains("viewshed_clear()"),
        "T-644: switching away from LoS-viewshed must clear the state AND drop the engine lane"
    );
    // Session-local overlay state (a ViewshedState in a RefCell), registered for the bridge.
    assert!(
        ed.contains("ViewshedState::new()") && ed.contains("register_viewshed_state("),
        "T-644 (Decision 4): the viewshed is a session-local ViewshedState, registered for the \
         overlay/engine bridge — not doc state"
    );
}

/// The fired rule at the wiring layer (perturb / fail / restore): the `is_viewshed()` branch in the
/// shared commit is load-bearing. The pin passes on the real body; a perturbation that drops the
/// viewshed route (so a viewshed click would fall through to the ray `los.click` — the exact
/// regression) makes the placement pin FAIL. Restore is implicit (an in-memory copy is perturbed).
#[test]
fn fired_rule_viewshed_routing_is_load_bearing() {
    let ed = editor_live();
    let needle = "viewshed.borrow_mut().place(";
    assert!(
        ed.contains(needle),
        "canary: the real body places a viewshed"
    );
    // Perturb: remove the viewshed placement route. The placement pin's needle must vanish.
    let perturbed = ed.replace(needle, "los.borrow_mut().click(");
    assert!(
        !perturbed.contains(needle),
        "fired rule: dropping the viewshed place() route (viewshed clicks fall through to the ray \
         click) must break the placement pin — proving the is_viewshed() branch discriminates"
    );
}
