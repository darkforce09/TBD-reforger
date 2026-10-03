//! The render statistics: the zero state, the skipped-frame flag and the per-lane counts.

use crate::render_stats::RenderStats;
use render_primitives::frame::ids::LaneId;

#[test]
fn a_new_renderer_has_submitted_nothing_and_counted_nothing() {
    let stats = RenderStats::new();
    assert!(!stats.submitted_last_frame());
    assert_eq!(stats.render_cpu_ms_last(), 0.0);
    assert_eq!(stats.render_cpu_ms_ema(), 0.0);
    assert_eq!(stats.lane_count(LaneId(0)), 0);
    assert_eq!(stats, RenderStats::default());
}

#[test]
fn a_skipped_frame_clears_the_submitted_flag_and_keeps_the_cpu_figures() {
    let mut stats = RenderStats::new();
    stats.record_skipped_frame();
    assert!(!stats.submitted_last_frame());
    assert_eq!(stats.render_cpu_ms_last(), 0.0);
    assert_eq!(stats.render_cpu_ms_ema(), 0.0);
}

#[test]
fn a_lane_count_is_kept_per_lane_and_overwritten_by_the_next_report() {
    let mut stats = RenderStats::new();
    stats.set_lane_count(LaneId(6), 120);
    stats.set_lane_count(LaneId(12), 7);
    assert_eq!(stats.lane_count(LaneId(6)), 120);
    assert_eq!(stats.lane_count(LaneId(12)), 7);
    assert_eq!(
        stats.lane_count(LaneId(7)),
        0,
        "a neighbouring lane is untouched"
    );

    stats.set_lane_count(LaneId(6), 0);
    assert_eq!(stats.lane_count(LaneId(6)), 0);
    assert_eq!(stats.lane_count(LaneId(12)), 7);
}
