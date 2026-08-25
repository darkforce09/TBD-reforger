use crate::editor::arsenal::class_r_scrub::{live_code, only_body};

const HIST: &str = include_str!("../state/history.rs");

fn hist_live() -> String {
    live_code(HIST)
}

fn markers_bind_needle() -> String {
    format!("{}{}", "markers", "_bind")
}

fn marker_lane_xy_tints_needle() -> String {
    format!("{}{}", "marker_lane_xy_", "tints")
}

/// Both post-doc engine feeds must call `markers_bind`, packing args via `marker_lane_xy_tints`
/// (the sole builder). Deleting either feed line must turn this pin RED — lane-order pins in
/// map-engine-render never examine `mission_history`.
#[test]
fn rebind_and_after_doc_change_both_feed_markers_bind() {
    let hist = hist_live();
    let rebind = only_body(&hist, "pub fn rebind_engine_from_doc");
    let after = only_body(&hist, "fn after_doc_change");
    let bind = markers_bind_needle();
    let pack = marker_lane_xy_tints_needle();

    assert!(
        rebind.contains(&bind),
        "T-760: rebind_engine_from_doc must call markers_bind; body:\n{rebind}"
    );
    assert!(
        rebind.contains(&pack),
        "T-760: rebind_engine_from_doc must pack via marker_lane_xy_tints; body:\n{rebind}"
    );
    assert!(
        after.contains(&bind),
        "T-760: after_doc_change must call markers_bind; body:\n{after}"
    );
    assert!(
        after.contains(&pack),
        "T-760: after_doc_change must pack via marker_lane_xy_tints; body:\n{after}"
    );
}
