use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

fn attrs_src() -> String {
    live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )))
}

/* ─────────── T-700 3DEN-PLACE-013 — the numeric nudge ─────────── */

/// The step scale, exercised by CALLING it — the whole reason [`super::nudge_step`] lives
/// outside the wasm block. Two properties, and the second is the one a refactor breaks:
///
///  1. each modifier alone selects its own step, and bare PageUp is 1;
///  2. the scale is FIRST-MATCH and finest-first, so **no combination of modifiers can produce
///     a step larger than the largest single modifier** — the safety argument the doc comment
///     makes. A multiplicative rewrite (`Shift`×`Alt` = 1000) fails this outright.
#[test]
fn nudge_step_is_first_match_finest_first_and_never_compounds() {
    use super::nudge_step;
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
    // Finest held wins, whichever else is down.
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
    // The property, over the whole 2^3 space: a combo never out-steps the single modifiers.
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

/// [`super::nudged`] — direction, quantisation, and the refusal that protects a multi-edit.
#[test]
fn a_nudge_quantises_and_refuses_a_field_with_no_truthful_base() {
    use super::{nudge_step, nudged};
    assert_eq!(nudged(Some(12.0), true, 1.0), Some(13.0));
    assert_eq!(nudged(Some(12.0), false, 1.0), Some(11.0));
    assert_eq!(nudged(Some(12.0), true, 10.0), Some(22.0));
    assert_eq!(nudged(Some(-3.0), false, 100.0), Some(-103.0));
    // A field the selection DISAGREES on renders empty; `"".parse::<f64>()` is Err, so the
    // caller hands us None and there must be NO write. Nudging from an implied 0 would stamp
    // an absolute number onto every selected entity.
    assert_eq!(
        nudged(None, true, 1.0),
        None,
        "an empty (multi-value) field has no base to be relative to — refuse the nudge"
    );
    assert_eq!(nudged(Some(f64::NAN), true, 1.0), None);
    assert_eq!(nudged(Some(f64::INFINITY), true, 1.0), None);
    // Quantisation: ten fine nudges off zero must land on 1, not 0.9999999999999999.
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

/// The wiring, pinned where `cargo test` cannot reach: `number_field`'s keydown must consult
/// the SAME gate the typed path does before it touches the draft.
///
/// The ORDER is the assertion. `gate.locked_now()` has to be checked before `nudged(` is even
/// called — a guard placed after the arithmetic would still be a guard, but one refactor away
/// from writing first and asking later. And `locked_now` must carry `shut` first and
/// unconditionally, exactly like the reactive `locked`, so the keyboard cannot become a second
/// laxer opinion of "dead field" (the F-7 lie, re-told through a different input path).
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

/// A nudge writes the DRAFT, never the document — the coalescing decision, stated as code.
///
/// `number_field` must hold exactly ONE `on_commit(` call site, the one inside `commit`, so a
/// burst of nudges cannot mint one undo step (and one modal re-render) per keypress. This is
/// the assertion that goes red the moment someone "improves" the nudge into a live commit.
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

/// The keys themselves. `live_source` KEEPS string literals — `ev.key()` is compared against
/// literals, so this is the one pin that can see which keys are actually bound, and it must
/// not be satisfiable by the doc comment that describes them.
///
/// **wave-127 F-1 — the arrow keys and `step="any"` are part of this pin now.** A `type="number"`
/// input with no `step` gets `step=1` on step base `0`, and the WHATWG step-up algorithm SNAPS an
/// off-grid value onto the grid rather than adding to it: the browser's own ArrowUp on a focused
/// `412.37` set the DOM value to `413`, fired `input`, and blur committed the integer — T-775's
/// defect on the adjacent key. Losing EITHER half (the attribute or the interception) hands the
/// arrows back to the browser, so both are asserted here.
#[test]
fn the_nudge_is_bound_to_the_page_and_arrow_keys_and_never_steps_on_a_grid() {
    let src = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let field = only_body(&src, "fn number_field(");
    for key in ["\"PageUp\"", "\"PageDown\"", "\"ArrowUp\"", "\"ArrowDown\""] {
        assert!(field.contains(key), "number_field must bind {key}");
    }
    // The arrows must share the nudge's OWN match arms — bound to some other handler they would
    // not take `nudge_step`, and (worse) might not prevent the browser's default stepping.
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
    // All three modifiers reach the step scale, in the argument order `nudge_step` declares.
    assert!(
        field.contains("nudge_step(ev.ctrl_key(), ev.shift_key(), ev.alt_key())"),
        "the step must be scaled from the live modifier state, in (ctrl, shift, alt) order"
    );
}

/// **T-775** — [`super::field_display`] is presentation, and presentation is allowed to round
/// only because nothing commits it. What it may NOT do is what the old `format!("{}",
/// value.round())` did: report an authored `412.37` as the integer `412`.
#[test]
fn the_display_keeps_the_working_resolution_and_never_flattens_to_an_integer() {
    use super::{field_display, nudge_step, nudged};
    // The ticket's own repro value. This assertion IS the bug.
    assert_eq!(
        field_display(412.37),
        "412.37",
        "an authored coordinate must not be displayed as an integer"
    );
    assert_eq!(field_display(412.371), "412.371");
    assert_eq!(field_display(-45.5), "-45.5");
    // Whole numbers stay tidy — the tidiness the old `.round()` was reaching for, kept.
    assert_eq!(field_display(412.0), "412");
    assert_eq!(field_display(4120.0), "4120");
    assert_eq!(field_display(0.0), "0");
    // No field may claim a negative zero.
    assert_eq!(field_display(-0.0), "0");
    assert_eq!(field_display(-0.0001), "0");
    // The display's precision is `nudged`'s quantum, so a nudged value always survives it
    // verbatim — the keyboard can never produce a number its own field cannot show.
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
    // Past the working resolution the display DOES round — which is precisely why focus seeds
    // the draft from `exact` instead of from this string. Pinned below.
    assert_eq!(field_display(412.371_234_5), "412.371");
}

/// **T-775** — the two halves that make focusing and leaving a field a genuine no-op.
///
/// A source pin because `number_field` is `#[cfg(target_arch = "wasm32")]` and `cargo test`
/// cannot build it. Each half is useless without the other:
///   * FOCUS seeds the draft from `exact`, the full round-trip printing — not from the rounded
///     presentation. Seeding from the display is what made T-700's PageUp on `412.37` commit
///     `413` instead of `413.37`: the nudge inherited a rounding it never performed.
///   * BLUR skips `on_commit` when the parsed draft still equals the settled value. Nothing
///     downstream will do this for it — `engine_ops::attrs_update_position` writes and calls
///     `after_local_edit()` on every non-refused slot, so without this an idle click dirties the
///     mission and mints an undo step for a number nobody touched. (It also used to flatten a
///     manually authored Z; that is fixed at the caller now — wave-127 F-2, pinned below.)
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
    // `live_source` KEEPS literals: `format!("{value}")` is the assertion here, and `live_code`
    // blanks exactly the part that distinguishes it from a rounded format string.
    let live_src = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
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
    // wave-127 F-3 — the pin's job is now WIRING only: that `commit` asks `should_commit`, with
    // the gate's verdict and the settled value, before it writes. What the answer should BE is
    // decided by `should_commit_writes_only_a_new_finite_value` below, which CALLS the function.
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

/// **wave-127 F-3** — the blur/Enter decision, EXERCISED. Previously this lived as an inline
/// expression inside a wasm-only `view!` closure and was guarded by a source pin on its literal
/// text; with no wasm-bindgen-test harness in the repo, nothing ran it. A semantically identical
/// rewrite turned that pin red and a subtly wrong one kept it green — the exact inversion of what
/// a test is for.
#[test]
fn should_commit_writes_only_a_new_finite_value() {
    use super::should_commit;
    // The T-775 case: focus and leave an untouched field ⇒ no write, no undo step.
    assert!(
        !should_commit(false, 412.37, 412.37),
        "an idle focus/blur on an unchanged value must not commit"
    );
    assert!(!should_commit(false, 0.0, 0.0));
    // A real edit commits, however small — 3 decimals is the working resolution.
    assert!(should_commit(false, 412.371, 412.37));
    assert!(should_commit(false, -1.0, 1.0));
    // A DIFFERING field is exempt from the equality skip: under a multi-selection the settled
    // value is one arbitrary member's number, so typing it is a deliberate stamp on the rest.
    assert!(
        should_commit(true, 412.37, 412.37),
        "a differing (multi-value) field must commit even when the typed number equals the one \
             arbitrary member's value it was compared against"
    );
    assert!(should_commit(true, 5.0, 9.0));
    // Non-finite refuses under BOTH gate verdicts. `"inf"`/`"NaN"` parse as f64 and the core
    // filters them per axis, but a refused write still fires `after_local_edit()` at the caller
    // — a dirty mission and an undo step for a number that never landed.
    for differs in [false, true] {
        for n in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(
                !should_commit(differs, n, 1.0),
                "a non-finite draft must never commit (differs={differs}, n={n})"
            );
        }
    }
    // NaN is not equal to itself, so the equality skip alone would have LET IT THROUGH — the
    // finite check has to be its own rule, not a consequence of the comparison.
    assert!(
        !should_commit(false, f64::NAN, f64::NAN),
        "NaN != NaN, so the equality skip cannot be what stops a non-finite draft"
    );
}

/// **T-785 — `text_field` commits on BLUR/ENTER, never per keystroke.** This is the whole bug.
///
/// A source pin because `text_field` is `#[cfg(target_arch = "wasm32")]` and there is no
/// wasm-bindgen-test harness in this repo — the same reason `number_field`'s behaviour is pinned
/// this way above. The defect it guards: `on:input=move |ev| on_change(...)` committed one store
/// round-trip per character, each bumping `doc_tick`, which re-rendered the `AttributesModal`
/// body and RE-CREATED this input mid-word. Focus fell to `<body>` and the tail of the word ran
/// as editor chords. The fix is the `number_field` shape: a `focused`/`draft` split, `on:input`
/// writing the DRAFT only, and the commit on `on:blur` / Enter.
///
/// The assertions are the anti-regression: `on_change(` must NOT be reachable from `on:input`,
/// and it must be reachable from the blur `commit` closure. `on:input` writing `draft.set(` is
/// the positive half — a `text_field` with no `on:input` at all would pass a naive "no commit on
/// input" check while making the field un-typeable.
#[test]
fn text_field_commits_on_blur_or_enter_and_never_remounts_mid_keystroke() {
    let src = attrs_src();
    let field = only_body(&src, "fn text_field(");
    // The remount cause, banned: the input handler must not commit. It writes the draft.
    assert!(
            field.contains("on:input=move |ev|")
                && field.contains("draft.set(event_target_value(&ev))")
                && field.contains("edited.set(true)"),
            "text_field's on:input must write the local DRAFT and set the edited latch, not commit;              body was:\n{field}"
        );
    // The commit lives behind blur/Enter, exactly like number_field. Exactly one `on_change(`
    // call site, and it is inside the `text_commit` closure the blur handler fires — a
    // per-keystroke commit is a per-keystroke remount against the modal's `doc_tick` re-render.
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
    // Enter commits by blurring — one seam shared with the blur path, never a second `on_change`.
    // The key is a string LITERAL, so it is read from the literal-kept half (`attrs_src` /
    // `live_code` blanks it); `.blur()` is code and survives either way.
    let live_src = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let live_field = only_body(&live_src, "fn text_field(");
    assert!(
        live_field.contains("\"Enter\" =>") && field.contains(".blur()"),
        "Enter must commit by blurring the input (the shared seam), not by a second commit call"
    );
    // The focused/draft split itself: while focused the input shows the draft, and focus seeds
    // the draft. Without this the input is a plain `value=` again and the remount returns.
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

/// **T-785** — the `text_field` no-op skip and the multi-edit contract, pinned together.
///
/// An untouched focus/blur must not write (it would dirty the mission and mint an undo step, the
/// same way `number_field`'s did before T-775). A DIFFERING multi-value field is EXEMPT from that
/// skip — typing one member's string back is a deliberate stamp onto the whole selection — and it
/// stays `disabled` (locked) until "Apply to all" is ticked, which the review flagged must not
/// regress. `field_display`/`should_commit` are `number_field`'s; text has no parse, so the skip
/// is a direct string comparison against the settled value.
#[test]
fn text_field_skips_the_no_op_write_but_a_differing_field_still_stamps() {
    let src = attrs_src();
    let field = only_body(&src, "fn text_field(");
    let commit = only_body(&src, "let text_commit = move ||");
    // T-813 — an operator-edited latch gates the write. A differing field alone must NOT
    // commit (that was wave200 F3: focus+blur stamped "" across the selection). Real input
    // sets the latch; Escape clears it before blur.
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
    // The multi-value display is EMPTY, never one arbitrary member's string — same rule as
    // number_field, and the seed for focus reads the same `text_display()`.
    let display = only_body(&src, "let text_display = move ||");
    assert!(
        display.contains("gate.differs()") && display.contains("String::new()"),
        "a differing field must render EMPTY, not one member's value; body was:\n{display}"
    );
    // The lock stays: disabled while the gate is locked (multi-edit not opted in / refused).
    assert!(
            field.contains("disabled=move || gate.locked()"),
            "text_field must stay disabled while the gate is locked — the 'Multiple values'              locked-state the review said must not regress"
        );
    // T-813 / wave200 F6 — field Escape must consume so the modal does not close on abandon.
    let live = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
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

/// **T-785 — the chord guard reads the LIVE `document.activeElement` directly.** This is the last
/// line of defence F-26-root asked to harden: every editor chord (E/R docks, Space camera, G
/// snap, Ctrl+A, copy/paste) sits behind `mission_history::in_editable_field()`, so whatever it
/// returns decides "typed character" vs "chord". It must read the tag and contentEditable state
/// off `activeElement` at the moment of the keypress — never a cached "is a field open?" flag —
/// so a field that has lost focus can never keep swallowing keys, and a chord can never fire
/// while a field genuinely holds focus.
///
/// A cross-file source pin because `mission_history` is `#[cfg(target_arch = "wasm32")]` and does
/// not compile under native `cargo test`; `attributes` does, and reads it as a string here — the
/// same shape as the `editor_ops.rs` pins in this module.
#[test]
fn the_chord_guard_reads_active_element_tag_and_content_editable_directly() {
    let mh = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/bridge/document_host/history.rs"
    )));
    let body = only_body(&mh, "pub fn in_editable_field() -> bool");
    // The source of truth is the LIVE focused node, fetched every call.
    assert!(
        body.contains("active_element()"),
        "in_editable_field must read document.activeElement, not a cached flag; body:\n{body}"
    );
    // Native form controls by tag, and contentEditable hosts by property — both direct off the
    // element, so focus loss to <body> (neither) reads as "not editable" the instant it happens.
    assert!(
        body.contains("tag_name()") && body.contains("is_content_editable"),
        "in_editable_field must check the element tag AND contentEditable directly; body:\n{body}"
    );
    // It must NOT gate on a cached "is a field open?" flag — reading the live activeElement is
    // the whole point, so a field that has lost focus stops swallowing keys immediately. (The
    // editor keydown's own guard call is pinned by mission_editor's tests, which own that file.)
    assert!(
        !body.contains("attrs_open") && !body.contains("renaming"),
        "in_editable_field must not consult an 'is a field open' signal — read activeElement live"
    );
}
