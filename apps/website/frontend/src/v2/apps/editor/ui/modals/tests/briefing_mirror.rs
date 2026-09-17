use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};

/// The blank arm must call the clear mutator — early-return on empty was the wave-117 defect.
#[test]
fn clearing_a_briefing_calls_the_clear_mutator() {
    let src = live_code(include_str!("../settings_modal.rs"));
    let mirror = format!("mirror{}", "_briefing_into_document");
    let body = only_body(&src, &format!("fn {mirror}"));
    let clear = format!("clear{}", "_meta_briefing");
    let apply = format!("apply{}", "_row_meta");
    assert!(
        body.contains(&format!("{clear}(")),
        "T-766: blank briefing must call MissionDocCore::{clear}"
    );
    assert!(
        body.contains(&format!("{apply}(")),
        "T-766: non-blank path must still use apply_row_meta"
    );
    // The pre-fix early return: `if briefing.trim().is_empty() { return; }` — refuse it.
    // A hollow `return;` after the clear call would still be wrong if clear is unreachable;
    // require that trim-empty leads to the clear call (clear appears after is_empty check).
    let empty = format!("is{}", "_empty");
    let trim_empty_idx = body
        .find(&format!("trim().{empty}()"))
        .expect("T-766: mirror must still branch on trim().is_empty()");
    let clear_idx = body.find(&format!("{clear}(")).expect("clear call");
    assert!(
        clear_idx > trim_empty_idx,
        "T-766: clear_meta_briefing must be on the empty arm, not before the trim check"
    );
}

/// Wave 133 F1 — the mirror body's clear-on-empty is load-bearing, but blank `next` must still
/// *reach* the call from `set_presentation`'s Ok/Briefing arm. Wrapping
/// `mirror_briefing_into_document(&next)` in `if !next.trim().is_empty() { … }` greens the body
/// pin while restoring the wave-117 defect (row PATCHes `""`, `meta.briefing` keeps the deleted
/// text). Pin reachability: no trim-empty gate between `Briefing =>` and the call.
#[test]
fn blank_next_reaches_the_mirror_at_the_ok_briefing_arm() {
    let src = live_code(include_str!("../settings_modal.rs"));
    let mirror = format!("mirror{}", "_briefing_into_document");
    let set = only_body(&src, &format!("fn set{}", "_presentation"));
    let call = format!("{mirror}(&next)");
    let call_idx = set
        .find(&call)
        .expect("T-766: Ok/Briefing arm must call mirror_briefing_into_document(&next)");
    let briefing = format!("Presentation{}::Briefing", "Field");
    let arm_idx = set[..call_idx]
        .rfind(&briefing)
        .expect("T-766: mirror call must sit in the PresentationField::Briefing arm");
    let window = &set[arm_idx..call_idx];
    let empty = format!("is{}", "_empty");
    assert!(
        !window.contains(&format!("trim().{empty}()")),
        "T-766 / wave 133 F1: blank next must still reach the mirror — no trim().is_empty gate between Briefing => and the mirror call"
    );
}
