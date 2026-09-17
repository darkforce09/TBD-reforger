//! Validation panel top bar findings chip tests.

use super::{
    chip_findings, findings_dropdown, register_panel_sink, PanelFinding, INITIAL_EVAL_MAX_TICKS,
};
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};
use leptos::prelude::*;
use website_map_engine::data::scenario::validate::Primitive;
use website_map_engine::data::scenario::validate::Severity;

#[test]
fn chip_findings_returns_the_registered_sink() {
    let owner = Owner::new();
    owner.with(|| {
        let sig = RwSignal::new(Vec::<PanelFinding>::new());
        register_panel_sink(sig);
        let got = chip_findings().expect("the sink must be registered");
        assert!(got.get_untracked().is_empty());
        sig.set(vec![PanelFinding {
            rule_id: "V1-PLAYER-SPAWN".into(),
            severity: Severity::Error,
            primitive: Primitive::RequiredEntity,
            message: "m".into(),
            subject: "s".into(),
            subject_id: None,
        }]);
        let got = chip_findings().expect("still registered");
        assert_eq!(
            got.get_untracked().len(),
            1,
            "chip_findings observes the sink's write"
        );
    });
}

#[test]
fn load_time_eval_retries_until_the_source_is_ready() {
    let code = live_code(super::VALIDATION_PANEL_SOURCE);
    let body = only_body(&code, "pub fn ValidationPanel(");
    for needle in ["read_payload_source()", "INITIAL_EVAL_MAX_TICKS"] {
        assert!(
            body.contains(needle),
            "T-798 (a): the load-time pass must retry until the source is ready — missing \
                 `{needle}`. Hollow: collapse it back to one `set_timeout(run_eval, 0)` → RED, and \
                 t0 shows 'No issues' until the first edit."
        );
    }
    assert!(
        body.contains("!ready && n < INITIAL_EVAL_MAX_TICKS"),
        "T-798 (a): the poll must reschedule on the bounded not-ready condition \
             `!ready && n < INITIAL_EVAL_MAX_TICKS`. Hollow: remove the reschedule → single-shot → \
             RED, and t0 shows 'No issues' until the first edit."
    );
    let guard_at = body
        .find("!ready && n < INITIAL_EVAL_MAX_TICKS")
        .expect("reschedule guard present");
    let mut end = (guard_at + 400).min(body.len());
    while end < body.len() && !body.is_char_boundary(end) {
        end += 1;
    }
    let region = &body[guard_at..end];
    assert!(
            region.contains("set_timeout") && region.contains("next"),
            "T-798 (a): the not-ready branch must `set_timeout(... next() ...)` — re-arm the poll by \
             calling itself (`next` is the self-reschedule handle). Without it the guard is inert and \
             t0 never converges."
        );
    assert!(
        INITIAL_EVAL_MAX_TICKS > 0,
        "the poll must have a positive tick budget"
    );
    assert!(
        INITIAL_EVAL_MAX_TICKS <= 200,
        "the poll must be BOUNDED, not an unbounded spin (host build never gets a source)"
    );
}

#[test]
fn the_floating_card_is_gone() {
    let src = live_source(super::VALIDATION_PANEL_SOURCE);
    for banned in [
        "bottom-14 left-3",
        "data-validation-panel",
        "data-validation-rollup",
    ] {
        assert!(
            !src.contains(banned),
            "T-798: the retired floating card's `{banned}` must not survive — the chip in \
                 `eden_top_strip` is the surface now, and it rides the strip's chrome_hidden gate."
        );
    }
}

#[test]
fn the_dropdown_keeps_the_v1_copy_and_the_legend() {
    let code = live_code(super::VALIDATION_PANEL_SOURCE);
    let body = only_body(&code, "pub fn findings_dropdown(");
    for needle in ["group_list_view", "legend_view"] {
        assert!(
            body.contains(needle),
            "T-798: findings_dropdown must render the grouped list AND the legend (`{needle}`) — \
                 the review pinned the legend content into the dropdown."
        );
    }
    let lit = live_source(super::VALIDATION_PANEL_SOURCE);
    let body_lit = only_body(&lit, "pub fn findings_dropdown(");
    assert!(
        body_lit.contains("No issues"),
        "T-798: the dropdown's empty state stays the quiet 'No issues' (the ticket's empty-state \
             call), not a 0-badge."
    );
}

#[test]
fn findings_dropdown_renders_both_states() {
    let owner = Owner::new();
    owner.with(|| {
        let _empty = findings_dropdown(Vec::new());
        let _full = findings_dropdown(vec![PanelFinding {
            rule_id: "V1-PLAYER-SPAWN".into(),
            severity: Severity::Error,
            primitive: Primitive::RequiredEntity,
            message: "This mission declares a faction but has no slots".into(),
            subject: "/editor/slots".into(),
            subject_id: None,
        }]);
    });
}
