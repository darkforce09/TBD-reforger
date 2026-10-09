//! Attributes modal numeric field input tests.

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

/// The Transform tab's Z field commits through `commit_position`; the authored height lands in the
/// hosted document exactly, for one slot and for an apply-to-all selection, and leaves the
/// untouched X/Y columns where they were.
#[test]
fn a_transform_commit_carries_z_into_the_document() {
    use super::attribute_commits_and_revert::commit_position;
    use leptos::prelude::StoredValue;
    use mission_document::MissionDocCore;
    use mission_editing_session::host::{install, with_doc};
    use std::cell::RefCell;
    use std::rc::Rc;

    let core = MissionDocCore::new();
    core.set_origin_init(true);
    core.add_slot(
        "a", "sq", "layer", 0, "Rifleman", None, None, 100.0, 200.0, 0.0, 0.0,
    );
    core.add_slot(
        "b", "sq", "layer", 1, "Rifleman", None, None, 300.0, 400.0, 5.0, 0.0,
    );
    core.set_origin_init(false);
    install(
        Rc::new(RefCell::new(Some(core))),
        Rc::new(RefCell::new(Vec::new())),
    );
    let position = |id: &str| -> serde_json::Value {
        with_doc(|c| serde_json::from_str::<serde_json::Value>(&c.slots_json()).unwrap())
            .expect("a hosted document")[id]["position"]
            .clone()
    };

    commit_position(
        StoredValue::new(vec!["a".to_string()]),
        None,
        None,
        Some(42.125),
        None,
    );
    let a = position("a");
    assert_eq!(
        a["z"], 42.125,
        "the single-slot commit must store the typed Z"
    );
    assert_eq!(
        (a["x"].as_f64(), a["y"].as_f64()),
        (Some(100.0), Some(200.0))
    );
    assert_eq!(
        position("b")["z"],
        5.0,
        "an unselected slot keeps its height"
    );

    commit_position(
        StoredValue::new(vec!["a".to_string(), "b".to_string()]),
        None,
        None,
        Some(-3.5),
        None,
    );
    for (id, x, y) in [("a", 100.0, 200.0), ("b", 300.0, 400.0)] {
        let p = position(id);
        assert_eq!(p["z"], -3.5, "apply-to-all must stamp the Z on `{id}`");
        assert_eq!((p["x"].as_f64(), p["y"].as_f64()), (Some(x), Some(y)));
    }
}
