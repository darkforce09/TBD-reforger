//! Attributes modal numeric field input tests.

use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

fn attrs_src() -> String {
    live_code(super::ATTRIBUTES_MODAL_SOURCE)
}

#[test]
fn nudge_step_is_first_match_finest_first_and_never_compounds() {
    use super::field_inputs::nudge_step;
    assert_eq!(nudge_step(false, false, false), 1.0, "bare PageUp is 1");
    assert_eq!(nudge_step(true, false, false), 0.1, "Ctrl is the fine step");
    assert_eq!(
        nudge_step(false, true, false),
        10.0,
        "Shift is the coarse step"
    );
    assert_eq!(
        nudge_step(false, false, true),
        100.0,
        "Alt is the coarsest step"
    );
    assert_eq!(
        nudge_step(true, true, false),
        0.1,
        "Ctrl+Shift takes Ctrl's step"
    );
    assert_eq!(
        nudge_step(true, false, true),
        0.1,
        "Ctrl+Alt takes Ctrl's step"
    );
    assert_eq!(
        nudge_step(true, true, true),
        0.1,
        "all three take Ctrl's step"
    );
    assert_eq!(
        nudge_step(false, true, true),
        10.0,
        "Shift+Alt takes Shift's step"
    );
    let solo: f64 = [
        nudge_step(true, false, false),
        nudge_step(false, true, false),
        nudge_step(false, false, true),
        nudge_step(false, false, false),
    ]
    .into_iter()
    .fold(0.0, f64::max);
    for ctrl in [false, true] {
        for shift in [false, true] {
            for alt in [false, true] {
                let s = nudge_step(ctrl, shift, alt);
                assert!(
                    s <= solo,
                    "ctrl={ctrl} shift={shift} alt={alt} stepped {s}, larger than the biggest \
                         single-modifier step {solo} — the scale has started compounding"
                );
                assert!(s > 0.0, "a step must move the value");
            }
        }
    }
}

#[test]
fn a_nudge_quantises_and_refuses_a_field_with_no_truthful_base() {
    use super::field_inputs::{nudge_step, nudged};
    assert_eq!(nudged(Some(12.0), true, 1.0), Some(13.0));
    assert_eq!(nudged(Some(12.0), false, 1.0), Some(11.0));
    assert_eq!(nudged(Some(12.0), true, 10.0), Some(22.0));
    assert_eq!(nudged(Some(-3.0), false, 100.0), Some(-103.0));
    assert_eq!(
        nudged(None, true, 1.0),
        None,
        "an empty (multi-value) field has no base to be relative to — refuse the nudge"
    );
    assert_eq!(nudged(Some(f64::NAN), true, 1.0), None);
    assert_eq!(nudged(Some(f64::INFINITY), true, 1.0), None);
    let fine = nudge_step(true, false, false);
    let mut v = 0.0_f64;
    for _ in 0..10 {
        v = nudged(Some(v), true, fine).expect("a finite base nudges");
    }
    assert_eq!(
        v, 1.0,
        "ten Ctrl nudges off zero must land exactly on 1.0, got {v}"
    );
    assert_eq!(
        format!("{v}"),
        "1",
        "the quantised value is what the field displays and commits"
    );
}

#[test]
fn the_nudge_takes_the_same_refusal_gate_as_a_typed_edit() {
    let src = attrs_src();
    let field = only_body(&src, "fn number_field(");
    let guard = field
        .find("gate.locked_now()")
        .expect("number_field's keydown must consult the gate before nudging");
    let arith = field
        .find("nudged(")
        .expect("number_field must call the nudge arithmetic");
    assert!(
        guard < arith,
        "the refusal must be checked BEFORE the nudge is computed; guard at {guard}, \
             nudged( at {arith}"
    );
    let now = only_body(&src, "fn locked_now(self) -> bool");
    assert!(
        now.contains("self.shut || self.opt.is_some_and(|o| !o.get_untracked())"),
        "locked_now must short-circuit on `shut` exactly as `locked` does; body was:\n{now}"
    );
}

#[test]
fn a_nudge_writes_the_draft_and_leaves_the_commit_to_blur_or_enter() {
    let src = attrs_src();
    let field = only_body(&src, "fn number_field(");
    assert_eq!(
        field.matches("on_commit(").count(),
        1,
        "number_field must commit from exactly one place (the `commit` closure); a per-nudge \
             commit is one undo step per keypress against `capture_timeout_millis = 0`"
    );
    let commit = only_body(&src, "let commit = move ||");
    assert!(
        commit.contains("on_commit(n)"),
        "the single commit site is the blur/Enter closure; body was:\n{commit}"
    );
    assert!(
        field.contains("draft.set(format!("),
        "the nudge's only write is the local draft"
    );
}

#[test]
fn the_nudge_is_bound_to_the_page_and_arrow_keys_and_never_steps_on_a_grid() {
    let src = live_source(super::ATTRIBUTES_MODAL_SOURCE);
    let field = only_body(&src, "fn number_field(");
    for key in ["\"PageUp\"", "\"PageDown\"", "\"ArrowUp\"", "\"ArrowDown\""] {
        assert!(field.contains(key), "number_field must bind {key}");
    }
    for arm in [
        "\"PageUp\" | \"ArrowUp\" => true,",
        "\"PageDown\" | \"ArrowDown\" => false,",
    ] {
        assert!(
            field.contains(arm),
            "the arrow keys must enter the SAME nudge as the page keys; missing arm `{arm}` in \
                 body:\n{field}"
        );
    }
    assert!(
        field.contains("step=\"any\""),
        "the input must carry step=\"any\": with the default step=1 the browser's own arrow \
             keys and spinner snap an authored 412.37 onto the integer grid"
    );
    assert!(
        field.contains("ev.prevent_default()"),
        "PageUp/PageDown scroll by default and the arrows step by default — the nudge must \
             claim the key or the modal moves (or the value rounds) instead of the number"
    );
    assert!(
        field.contains("nudge_step(ev.ctrl_key(), ev.shift_key(), ev.alt_key())"),
        "the step must be scaled from the live modifier state, in (ctrl, shift, alt) order"
    );
}

#[test]
fn the_display_keeps_the_working_resolution_and_never_flattens_to_an_integer() {
    use super::field_inputs::{field_display, nudge_step, nudged};
    assert_eq!(
        field_display(412.37),
        "412.37",
        "an authored coordinate must not be displayed as an integer"
    );
    assert_eq!(field_display(412.371), "412.371");
    assert_eq!(field_display(-45.5), "-45.5");
    assert_eq!(field_display(412.0), "412");
    assert_eq!(field_display(4120.0), "4120");
    assert_eq!(field_display(0.0), "0");
    assert_eq!(field_display(-0.0), "0");
    assert_eq!(field_display(-0.0001), "0");
    for step in [
        nudge_step(true, false, false),
        nudge_step(false, true, false),
        nudge_step(false, false, true),
        nudge_step(false, false, false),
    ] {
        let n = nudged(Some(412.37), true, step).expect("a finite base nudges");
        assert_eq!(
            field_display(n),
            format!("{n}"),
            "a nudge quantises to 3 decimals; the display must not round it further"
        );
    }
    assert_eq!(field_display(412.371_234_5), "412.371");
}

#[test]
fn an_untouched_field_commits_nothing_and_the_draft_seeds_from_the_exact_value() {
    let src = attrs_src();
    let field = only_body(&src, "fn number_field(");
    assert!(
        !field.contains("value.round()"),
        "the field must not round the authored value away — that was the T-775 defect"
    );
    assert!(
        field.contains("StoredValue::new(field_display(value))"),
        "the unfocused display must go through `field_display`"
    );
    let live_src = live_source(super::ATTRIBUTES_MODAL_SOURCE);
    let live = only_body(&live_src, "fn number_field(");
    assert!(
        live.contains("let exact = StoredValue::new(format!(\"{value}\"));"),
        "`exact` must be the full round-trip printing of the value; body was:\n{live}"
    );
    let seed = only_body(&src, "let seed = move ||");
    assert!(
        seed.contains("exact.get_value()") && !seed.contains("shown.get_value()"),
        "focus must seed the draft from the EXACT value, never the presentation; body was:\n\
             {seed}"
    );
    assert!(
        field.contains("draft.set(seed())"),
        "the focus handler is what puts the exact value into the draft"
    );
    let commit = only_body(&src, "let commit = move ||");
    assert!(
        commit.contains("should_commit(gate.differs(), n, value)"),
        "commit must route the decision through `should_commit`, handing it the multi-edit \
             gate's verdict and the SETTLED value; body was:\n{commit}"
    );
    let guard = commit
        .find("should_commit(")
        .expect("the skip must gate the commit, not merely be computed");
    let call = commit
        .find("on_commit(n)")
        .expect("commit must still hold its one write");
    assert!(
        guard < call,
        "the no-op skip must be checked BEFORE the write; guard at {guard}, on_commit(n) at \
             {call}"
    );
}

#[test]
fn should_commit_writes_only_a_new_finite_value() {
    use super::field_inputs::should_commit;
    assert!(
        !should_commit(false, 412.37, 412.37),
        "an idle focus/blur on an unchanged value must not commit"
    );
    assert!(!should_commit(false, 0.0, 0.0));
    assert!(should_commit(false, 412.371, 412.37));
    assert!(should_commit(false, -1.0, 1.0));
    assert!(
        should_commit(true, 412.37, 412.37),
        "a differing (multi-value) field must commit even when the typed number equals the one \
             arbitrary member's value it was compared against"
    );
    assert!(should_commit(true, 5.0, 9.0));
    for differs in [false, true] {
        for n in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(
                !should_commit(differs, n, 1.0),
                "a non-finite draft must never commit (differs={differs}, n={n})"
            );
        }
    }
    assert!(
        !should_commit(false, f64::NAN, f64::NAN),
        "NaN != NaN, so the equality skip cannot be what stops a non-finite draft"
    );
}

#[test]
fn text_field_commits_on_blur_or_enter_and_never_remounts_mid_keystroke() {
    let src = attrs_src();
    let field = only_body(&src, "fn text_field(");
    assert!(
            field.contains("on:input=move |ev|")
                && field.contains("draft.set(event_target_value(&ev))")
                && field.contains("edited.set(true)"),
            "text_field's on:input must write the local DRAFT and set the edited latch, not commit;              body was:\n{field}"
        );
    assert_eq!(
            field.matches("on_change(").count(),
            1,
            "text_field must commit from exactly one place (the blur/Enter `text_commit` closure); a \
             per-input commit is what remounted the input and dropped focus to <body>. Body:\n{field}"
        );
    let commit = only_body(&src, "let text_commit = move ||");
    assert!(
        commit.contains("on_change(next)"),
        "the single commit site is the blur/Enter closure; body was:\n{commit}"
    );
    let live_src = live_source(super::ATTRIBUTES_MODAL_SOURCE);
    let live_field = only_body(&live_src, "fn text_field(");
    assert!(
        live_field.contains("\"Enter\" =>") && field.contains(".blur()"),
        "Enter must commit by blurring the input (the shared seam), not by a second commit call"
    );
    assert!(
        field.contains(
            "prop:value=move || { if focused.get() { draft.get() } else { text_display() } }"
        ),
        "text_field must show the DRAFT while focused (the number_field split); body was:\n{field}"
    );
    assert!(
        field.contains("draft.set(text_display())") && field.contains("focused.set(true)"),
        "the focus handler must seed the draft and mark the field focused"
    );
}

#[test]
fn text_field_skips_the_no_op_write_but_a_differing_field_still_stamps() {
    let src = attrs_src();
    let field = only_body(&src, "fn text_field(");
    let commit = only_body(&src, "let text_commit = move ||");
    assert!(
        field.contains("let edited = RwSignal::new(false)")
            && field.contains("edited.set(true)")
            && field.contains("edited.set(false)"),
        "text_field must latch real input edits; body was:\n{field}"
    );
    assert!(
            commit.contains("if !edited.get_untracked()")
                && commit.contains("gate.differs() || next != settled.get_value()"),
            "commit must require the edited latch, then skip an unchanged value yet still stamp a              differing (multi-value) field once edited; body was:\n{commit}"
        );
    let display = only_body(&src, "let text_display = move ||");
    assert!(
        display.contains("gate.differs()") && display.contains("String::new()"),
        "a differing field must render EMPTY, not one member's value; body was:\n{display}"
    );
    assert!(
            field.contains("disabled=move || gate.locked()"),
            "text_field must stay disabled while the gate is locked — the 'Multiple values'              locked-state the review said must not regress"
        );
    let live = live_source(super::ATTRIBUTES_MODAL_SOURCE);
    let live_field = only_body(&live, "fn text_field(");
    assert!(
        live_field.contains("\"Escape\" =>") && live_field.contains("stop_propagation()"),
        "text_field Escape must stop_propagation so the modal stays open; body was:\n{live_field}"
    );
    let live_num = only_body(&live, "fn number_field(");
    assert!(
        live_num.contains("key == \"Escape\"") && live_num.contains("stop_propagation()"),
        "number_field Escape must stop_propagation (same family); body was:\n{live_num}"
    );
}

#[test]
fn the_chord_guard_reads_active_element_tag_and_content_editable_directly() {
    let mh = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/bridge/document_host/history.rs"
    )));
    let body = only_body(&mh, "pub fn in_editable_field() -> bool");
    assert!(
        body.contains("active_element()"),
        "in_editable_field must read document.activeElement, not a cached flag; body:\n{body}"
    );
    assert!(
        body.contains("tag_name()") && body.contains("is_content_editable"),
        "in_editable_field must check the element tag AND contentEditable directly; body:\n{body}"
    );
    assert!(
        !body.contains("attrs_open") && !body.contains("renaming"),
        "in_editable_field must not consult an 'is a field open' signal — read activeElement live"
    );
}
