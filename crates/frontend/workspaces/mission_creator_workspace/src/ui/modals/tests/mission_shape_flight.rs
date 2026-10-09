use super::ShapeSeq;

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
