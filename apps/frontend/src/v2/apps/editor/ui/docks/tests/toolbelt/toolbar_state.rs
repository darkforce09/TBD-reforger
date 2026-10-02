use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

/// The `cls` closure composes TOOL_BASE with the recipes — TOGGLED_PLATE for the current mode,
/// HOVER_FILL for the rest. Proven on scrubbed code so the needle is the real `cn` call.
#[test]
fn tool_states_consume_the_vocabulary_recipes() {
    let code = live_code(super::test_source::raw_toolbelt());
    let body = only_body(&code, &format!("pub fn {}", "ModeToolbar("));
    assert!(
        body.contains("TOGGLED_PLATE"),
        "the current tool must wear TOGGLED_PLATE (plate + dark top border)"
    );
    assert!(
        body.contains("HOVER_FILL"),
        "a live-but-not-current tool must wear HOVER_FILL"
    );
}

/// THE FIX, as an absence: the old ad-hoc tool states are gone — no `bg-primary/20` active tint
/// spelled inline outside the recipe, and no weaker `hover:bg-white/5` fill. TOOL_BASE carries
/// only geometry, and the states come from the recipes, so neither ad-hoc token appears as a
/// class literal in the mode toolbar. Checked on the string-kept source.
#[test]
fn no_ad_hoc_tool_state_classes_remain() {
    let src = live_source(super::test_source::raw_toolbelt());
    let mode = only_body(&src, &format!("pub fn {}", "ModeToolbar("));
    // The weaker ad-hoc hover fill the inactive tool used to wear.
    let weak_hover = ["hover:bg-", "white/5"].concat();
    assert!(
        !mode.contains(&weak_hover),
        "T-668: the toolbar's ad-hoc `hover:bg-white/5` must be gone (use HOVER_FILL's bg-white/10)"
    );
    // The active tint must not be spelled as a bare class inside the toolbar — it comes from
    // TOGGLED_PLATE now. (TOGGLED_PLATE's own definition lives in eden_layout, not here.)
    let active_tint = ["bg-", "primary/20"].concat();
    assert!(
        !mode.contains(&active_tint),
        "T-668: the active tool tint must come from TOGGLED_PLATE, not a bare bg-primary/20 here"
    );
}

/// Rule (3) — every mode-tool button keeps its `title` (Select / Ruler / LoS all carry one), so a
/// tool always explains itself. All three ship live today, so none is disabled, but the tooltip is
/// the same retention pattern rule 3 requires of a disabled control. Checked on the string-kept
/// source where the title literals survive.
#[test]
fn tools_keep_their_tooltips() {
    let src = live_source(super::test_source::raw_toolbelt());
    let mode = only_body(&src, &format!("pub fn {}", "ModeToolbar("));
    for tip in ["Select", "Ruler", "Line of sight"] {
        assert!(
            mode.contains(tip),
            "the {tip} tool button must carry its title (rule 3 tooltip retention)"
        );
    }
}
