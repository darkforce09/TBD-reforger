//! Zones panel tactical draw trigger tests.

use crate::v2::core::test_support::class_r_scrub::{live_code, live_source};

fn live_calls() -> String {
    live_code(super::ZONES_PANEL_SOURCE)
}
fn live_markup() -> String {
    live_source(super::ZONES_PANEL_SOURCE)
}

#[test]
fn the_panel_arms_the_tactical_draw() {
    let src = live_calls();
    assert!(
        src.contains("tactical_graphics_authoring::begin_tactical_draw("),
        "T-946.86 (.84): a production control must call begin_tactical_draw — the wave-255 \
             state was a complete draw tool with no way to start it"
    );
}

#[test]
fn the_trigger_is_a_button_with_a_stable_test_id() {
    let markup = live_markup();
    assert!(
        markup.contains("data-testid=\"tactical-draw-arm\""),
        "T-946.86 (.84): the arm needs a stable test id — the acceptance probe presses it"
    );
    let at_arm = markup
        .find("data-testid=\"tactical-draw-arm\"")
        .expect("checked above");
    let button_open = markup[..at_arm]
        .rfind("<button")
        .expect("T-946.86 (.84): the tactical arm must be a <button>, not a chord");
    assert!(
        !markup[button_open..at_arm].contains('>'),
        "T-946.86 (.84): the test id must sit on the button element itself"
    );
}

#[test]
fn an_armed_draw_can_be_seen_finished_and_abandoned() {
    let src = live_calls();
    for needle in [
        "tactical_graphics_authoring::tactical_draft()",
        "tactical_graphics_authoring::complete_tactical_draw()",
        "tactical_graphics_authoring::tactical_draw_pop_vertex()",
        "tactical_graphics_authoring::cancel_tactical_draw()",
    ] {
        assert!(
            src.contains(needle),
            "T-946.86 (.84): the tactical draw surface must reach `{needle}`"
        );
    }
}

#[test]
fn the_kind_list_is_the_cores_vocabulary() {
    let src = live_calls();
    assert!(
        src.contains("tactical_graphics::KINDS"),
        "T-946.86 (.84): the kind select must read map-engine-core's KINDS, not a local list"
    );
    for kind in website_map_engine::data::scenario::tactical_graphics::KINDS {
        assert!(
            website_map_engine::data::scenario::tactical_graphics::min_points(kind).is_some(),
            "every offered kind must be one begin_tactical_draw accepts — `{kind}` is not"
        );
    }
}
