//! Role: readback tests.
//! Position: `core/buffers/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::device::buffers::readback::ReadbackLane;

#[test]
fn a_second_claim_while_one_is_outstanding_is_refused() {
    let lane = ReadbackLane::new();
    assert!(lane.begin(), "the first claim always takes the lane");
    assert!(lane.in_flight());
    assert!(
        !lane.begin(),
        "a second map_async over a still-mapped buffer is the wgpu panic"
    );
    assert!(
        !lane.begin(),
        "and it stays refused — repeated frames must not each get a chance to crash"
    );
}

#[test]
fn a_failed_map_does_not_unmap_and_does_not_wedge_the_lane() {
    let lane = ReadbackLane::new();
    assert!(lane.begin());
    assert!(
        !lane.settle(false),
        "the failure arm must not unmap — the buffer was never mapped"
    );
    assert!(
        !lane.in_flight(),
        "a failed readback must still clear the flag"
    );
    assert!(lane.begin(), "…so the next frame can try again");
}

#[test]
fn a_successful_map_authorises_exactly_one_unmap() {
    let lane = ReadbackLane::new();
    assert!(lane.begin());
    assert!(lane.settle(true), "the success arm owns the unmap");
    assert!(!lane.in_flight());

    for _ in 0..3 {
        assert!(lane.begin());
        assert!(lane.settle(true));
    }
}

#[test]
fn a_failed_readback_retires_the_previous_sample() {
    let lane = ReadbackLane::new();

    assert!(lane.begin());
    assert!(lane.settle(true));
    lane.record_sample();
    assert!(lane.has_sample(), "the good reading is on offer");

    assert!(lane.begin());
    assert!(!lane.settle(false));
    assert!(
        !lane.has_sample(),
        "a stale number presented as a fresh one is worse than no number"
    );

    assert!(lane.begin());
    assert!(lane.settle(true));
    lane.record_sample();
    assert!(lane.has_sample());
}

#[test]
fn settling_ok_without_reading_advertises_nothing() {
    let lane = ReadbackLane::new();
    assert!(lane.begin());
    assert!(lane.settle(true));
    assert!(!lane.has_sample());
}
