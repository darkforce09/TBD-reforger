use super::ShapeSeq;
use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};

/// The reopen race: a GET that started (or would land) across a PATCH window must not apply.
#[test]
fn a_get_that_races_a_patch_cannot_apply() {
    let mut s = ShapeSeq::default();
    let load_gen = s.begin_load();
    assert!(s.may_apply_load(load_gen), "idle load may apply");
    s.begin_patch();
    assert!(
        !s.may_apply_load(load_gen),
        "GET captured before PATCH must not clobber the optimistic value"
    );
    assert_eq!(s.patch_inflight, 1);
    // Reopen mid-flight: capture current generation, but inflight blocks apply (and load skips GET).
    let reopen = s.begin_load();
    assert!(
        !s.may_apply_load(reopen),
        "reopen while PATCH in flight must not apply a pre-PATCH row"
    );
    s.end_patch();
    assert_eq!(s.patch_inflight, 0);
    assert!(
        !s.may_apply_load(load_gen),
        "GET that raced the PATCH window stays stale after settle"
    );
    assert!(
        !s.may_apply_load(reopen),
        "end_patch bumps generation so the mid-flight GET cannot land late"
    );
    let fresh = s.begin_load();
    assert!(s.may_apply_load(fresh), "a post-settle open may apply");
}

/// Production ShapeMirror must call the sequencer — not merely document it.
#[test]
fn shape_mirror_wires_the_sequencer() {
    let src = live_code(include_str!("../settings_modal.rs"));
    for (fn_name, needles) in [
        (
            "fn load",
            [
                "begin_load",
                "may_apply_load",
                "is_mission_row_id",
                "hydrated_row",
            ]
            .as_slice(),
        ),
        (
            "fn set_game_mode",
            [
                "begin_patch",
                "end_patch",
                "note_hydrated_game_mode",
                "is_mission_row_id",
            ]
            .as_slice(),
        ),
        (
            "fn set_presentation",
            [
                "begin_patch",
                "end_patch",
                "note_hydrated_presentation",
                "is_mission_row_id",
            ]
            .as_slice(),
        ),
    ] {
        let body = only_body(&src, fn_name);
        for needle in needles {
            assert!(
                body.contains(needle),
                "T-746: {fn_name} must call/use {needle}"
            );
        }
    }
    // Assembled so a comment saying the copy is gone cannot green this pin.
    let gone = format!("fn is{}", "_row_id");
    assert!(
        !src.contains(&gone),
        "T-746: the duplicated row-id copy must be gone"
    );
    assert!(
        src.contains("is_mission_row_id"),
        "T-746: ShapeMirror must use the shared row-id predicate"
    );
}
