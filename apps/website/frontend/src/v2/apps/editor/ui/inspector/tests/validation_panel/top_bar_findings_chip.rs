use super::{
    chip_findings, findings_dropdown, register_panel_sink, PanelFinding, INITIAL_EVAL_MAX_TICKS,
};
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};
use leptos::prelude::*;
use website_map_engine::data::scenario::validate::Primitive;
use website_map_engine::data::scenario::validate::Severity;

/// THE CHIP SEAM. `chip_findings()` must hand back the SAME signal `register_panel_sink` stored —
/// that is the whole coupling between the headless eval loop (which writes it) and the top-strip
/// chip (which reads it). Behavioural, under a real owner: register a signal, set it, and prove
/// `chip_findings()` returns a handle that observes the write. Hollow the seam (return `None`) →
/// the chip goes permanently blank and this goes RED.
#[test]
fn chip_findings_returns_the_registered_sink() {
    let owner = Owner::new();
    owner.with(|| {
        let sig = RwSignal::new(Vec::<PanelFinding>::new());
        register_panel_sink(sig);
        // Before any write, the chip sees an empty list (a clean mission → "No issues").
        let got = chip_findings().expect("the sink must be registered");
        assert!(got.get_untracked().is_empty());
        // A write to the underlying signal is visible through the seam handle — same signal.
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

/// LOAD-TIME EVAL (a). The initial pass must RE-RUN until the payload source resolves — a single
/// `set_timeout(0)` shows "No issues" until the first doc_tick because the doc hydrates async
/// (review F-11). The pin holds the retry shape in the component body: it re-checks the payload
/// source's readiness (`read_payload_source()`) and reschedules against the bounded
/// `INITIAL_EVAL_MAX_TICKS`. `live_code` blanks comments + strings so the prose describing F-11
/// cannot satisfy the needles.
#[test]
fn load_time_eval_retries_until_the_source_is_ready() {
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/validation_panel.rs"
    )));
    let body = only_body(&code, "pub fn ValidationPanel(");
    for needle in ["read_payload_source()", "INITIAL_EVAL_MAX_TICKS"] {
        assert!(
            body.contains(needle),
            "T-798 (a): the load-time pass must retry until the source is ready — missing \
                 `{needle}`. Hollow: collapse it back to one `set_timeout(run_eval, 0)` → RED, and \
                 t0 shows 'No issues' until the first edit."
        );
    }
    // THE RESCHEDULE ITSELF — the load-bearing structure. The poll re-arms only while the source
    // is NOT ready AND the budget has room: `!ready && n < INITIAL_EVAL_MAX_TICKS`, guarding a
    // `set_timeout` that calls `next()` (the self-reschedule). Deleting the reschedule (the
    // single-shot regression) removes this exact guard. This needle is ONLY in the reschedule
    // branch, so it cannot be satisfied by the debounce's own timer or the poll kickoff.
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
    // The budget is real and bounded (a host build has no source and must not spin forever).
    assert!(
        INITIAL_EVAL_MAX_TICKS > 0,
        "the poll must have a positive tick budget"
    );
    assert!(
        INITIAL_EVAL_MAX_TICKS <= 200,
        "the poll must be BOUNDED, not an unbounded spin (host build never gets a source)"
    );
}

/// THE FLOATING CARD IS RETIRED. The old bottom-left overlay (`absolute bottom-14 left-3`,
/// `data-validation-panel`) must be GONE — the visible surface is the top-strip chip now, and a
/// leftover card would defeat the F-35 fix (a card outside the strip's `chrome_hidden` gate would
/// survive Backspace again). `live_source` keeps class strings (the geometry is a literal).
#[test]
fn the_floating_card_is_gone() {
    let src = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/validation_panel.rs"
    )));
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

/// THE V1 COPY + THE LEGEND ARE PINNED VERBATIM, in the dropdown. The message copy is the best
/// writing in the product (operator + review agree) and must not drift; the severity legend
/// content stays (moved into the dropdown, not deleted). Rendered by `findings_dropdown`, so the
/// copy lives HERE. This asserts the copy is present in the module source (not scrubbed away).
#[test]
fn the_dropdown_keeps_the_v1_copy_and_the_legend() {
    // The V1 message is a string literal in the engine's rule, surfaced by this panel; the
    // panel's own pinned copy is the empty-state + the legend. `findings_dropdown` renders both.
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/validation_panel.rs"
    )));
    let body = only_body(&code, "pub fn findings_dropdown(");
    for needle in ["group_list_view", "legend_view"] {
        assert!(
            body.contains(needle),
            "T-798: findings_dropdown must render the grouped list AND the legend (`{needle}`) — \
                 the review pinned the legend content into the dropdown."
        );
    }
    // The quiet empty state (never a 0-badge / celebratory toast) is retained.
    let lit = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/validation_panel.rs"
    )));
    let body_lit = only_body(&lit, "pub fn findings_dropdown(");
    assert!(
        body_lit.contains("No issues"),
        "T-798: the dropdown's empty state stays the quiet 'No issues' (the ticket's empty-state \
             call), not a 0-badge."
    );
}

/// The `findings_dropdown` renders the empty state for a clean mission and the grouped list when
/// there are findings — a light behavioural check the two branches produce SOME view (the view
/// primitives are native-compilable, so this runs on the host).
#[test]
fn findings_dropdown_renders_both_states() {
    let owner = Owner::new();
    owner.with(|| {
        // Clean → empty state branch; findings → list branch. Both must build a view.
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
