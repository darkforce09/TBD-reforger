//! Role: draw order t748 comments bind feed.
//! Position: `overlay/tests/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

const HIST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../frontend/src/v2/apps/editor/bridge/document_host/history.rs"
));

fn only_body(src: &str, sig: &str) -> String {
    let start = src
        .find(sig)
        .unwrap_or_else(|| panic!("missing signature: {sig}"));
    let after = &src[start..];
    let brace = after.find('{').expect("missing body");
    let mut depth = 0usize;
    let mut end = brace;
    for (i, ch) in after[brace..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = brace + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    after[..end].to_string()
}

fn comments_bind_needle() -> String {
    format!("{}{}", "comments", "_bind")
}

fn comment_lane_xy_needle() -> String {
    format!("{}{}", "comment_lane_", "xy")
}

#[test]
fn rebind_and_after_doc_change_both_feed_comments_bind() {
    let rebind = only_body(HIST, "pub fn rebind_engine_from_doc");
    let after = only_body(HIST, "fn after_doc_change");
    let bind = comments_bind_needle();
    let pack = comment_lane_xy_needle();
    assert!(
        rebind.contains(&bind),
        "T-748: rebind_engine_from_doc must call comments_bind; body:\n{rebind}"
    );
    assert!(
        rebind.contains(&pack),
        "T-748: rebind_engine_from_doc must pack via comment_lane_xy; body:\n{rebind}"
    );
    assert!(
        after.contains(&bind),
        "T-748: after_doc_change must call comments_bind; body:\n{after}"
    );
    assert!(
        after.contains(&pack),
        "T-748: after_doc_change must pack via comment_lane_xy; body:\n{after}"
    );
}
