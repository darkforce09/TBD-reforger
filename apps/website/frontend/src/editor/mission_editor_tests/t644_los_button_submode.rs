use crate::editor::arsenal::class_r_scrub::{live_code, live_source, only_body};

/// (toggle on re-click) The LoS button's `on:pointerdown` toggles `los_mode` when LoS is ALREADY
/// active (`is_los()` true → `los_mode.update(… toggled())`) and otherwise sets `tool_mode = LoS`.
/// The `tool_mode.set(EditorTool::LoS)` still lives in the button (the honesty rule / t643 pin),
/// so the button never lies about which tool it selects.
#[test]
fn los_button_reclick_toggles_the_submode() {
    let code = live_code(include_str!("../panels/toolbelt.rs"));
    let body = only_body(&code, &format!("pub fn {}", "ModeToolbar("));
    assert!(
        body.contains("los_mode.update(|m| *m = m.toggled())"),
        "T-644: a re-click of the LoS button must toggle the sub-mode (LosMode::toggled)"
    );
    // The toggle is gated on LoS already being active (first click from another tool just
    // activates LoS; it does not advance the sub-mode).
    assert!(
        body.contains("tool_mode.get_untracked().is_los()"),
        "T-644: the toggle must fire only when LoS is already active (re-click semantics)"
    );
    // The set-LoS path is still present (t643 honesty rule — the button selects the tool it names).
    assert!(
        body.contains(&format!("tool_mode.set(EditorTool::{})", "LoS")),
        "T-644: the LoS button must still set tool_mode = LoS on the first (activate) click"
    );
}

/// (title/label reflect the sub-mode) The LoS button's title AND wide-layout label read the live
/// `los_mode` (`is_viewshed()`), so the operator always knows which sub-mode they're in. Proven on
/// the string-KEPT source (the title/label literals survive) so the needle is the real view text.
#[test]
fn los_button_reflects_the_active_submode() {
    let src = live_source(include_str!("../panels/toolbelt.rs"));
    let body = only_body(&src, &format!("pub fn {}", "ModeToolbar("));
    // The button reads the sub-mode to pick its title/label.
    assert!(
        body.matches("los_mode.get()").count() >= 1 && body.contains("is_viewshed()"),
        "T-644: the LoS button must read los_mode to reflect the active sub-mode"
    );
    // Both sub-mode words appear in the button's affordance (title + label).
    for word in ["viewshed", "ray"] {
        assert!(
            body.contains(word),
            "T-644: the LoS button title/label must name the {word} sub-mode"
        );
    }
    // t668/t642 retention: the base tooltip phrase survives (still explains the tool).
    assert!(
        body.contains("Line of sight"),
        "T-644: the LoS button must keep its 'Line of sight' title (tooltip retention)"
    );
}
