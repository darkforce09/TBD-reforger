use super::{
    CONN_LINE_RGBA, CONN_LINE_SELECTED_RGBA, ConnSegment, connection_lane_verts,
    connection_segments, pick_connection,
};
use std::collections::HashMap;

fn positions() -> HashMap<String, (f64, f64)> {
    [
        ("a".to_string(), (0.0, 0.0)),
        ("b".to_string(), (100.0, 0.0)),
        ("c".to_string(), (0.0, 100.0)),
    ]
    .into_iter()
    .collect()
}

fn rows(pairs: &[(&str, &str, &str)]) -> String {
    let arr: Vec<serde_json::Value> = pairs
        .iter()
        .map(|(id, from, to)| serde_json::json!({"id": id, "kind": "sync", "from": from, "to": to}))
        .collect();
    serde_json::Value::Array(arr).to_string()
}

/// A placed edge becomes one segment between its endpoints, in listing order.
#[test]
fn placed_edges_become_segments_in_listing_order() {
    let segs = connection_segments(&rows(&[("k2", "a", "b"), ("k1", "b", "c")]), &positions());
    assert_eq!(
        segs,
        vec![
            ConnSegment {
                id: "k2".into(),
                ax: 0.0,
                ay: 0.0,
                bx: 100.0,
                by: 0.0
            },
            ConnSegment {
                id: "k1".into(),
                ax: 100.0,
                ay: 0.0,
                bx: 0.0,
                by: 100.0
            },
        ]
    );
}

/// A DANGLING edge draws nothing — never a line to the origin. `CONN-DANGLING` is a panel
/// finding; a wrong line would be a second report that also lies about where the entity is.
/// A self-link (`CONN-SELF`) is dropped too: a zero-length segment is not a clickable artifact.
#[test]
fn dangling_and_self_edges_draw_nothing() {
    let segs = connection_segments(
        &rows(&[
            ("dangle-from", "ghost", "b"),
            ("dangle-to", "a", "ghost"),
            ("self", "a", "a"),
            ("good", "a", "b"),
        ]),
        &positions(),
    );
    assert_eq!(segs.len(), 1, "only the resolvable, non-self edge draws");
    assert_eq!(segs[0].id, "good");
    // Malformed input is inert, not a panic.
    assert!(connection_segments("not json", &positions()).is_empty());
    assert!(connection_segments("{}", &positions()).is_empty());
}

/// 6 floats per vertex, 2 vertices per segment, and the selected edge — and ONLY it — is tinted.
#[test]
fn lane_verts_tint_exactly_the_selected_edge() {
    let segs = connection_segments(&rows(&[("k1", "a", "b"), ("k2", "b", "c")]), &positions());
    let v = connection_lane_verts(&segs, Some("k2"));
    assert_eq!(v.len(), 2 * 12, "6 floats/vert, 2 verts/segment");
    assert_eq!((v[0], v[1]), (0.0, 0.0));
    assert_eq!((v[6], v[7]), (100.0, 0.0));
    assert_eq!(v[2..6], CONN_LINE_RGBA, "k1 is not selected");
    assert_eq!(v[8..12], CONN_LINE_RGBA, "both k1 verts share its colour");
    assert_eq!(v[14..18], CONN_LINE_SELECTED_RGBA, "k2 is selected");
    assert_eq!(v[20..24], CONN_LINE_SELECTED_RGBA);
    // No selection ⇒ nothing tinted; an id that names no edge ⇒ nothing tinted (a stale
    // selection whose edge was undone away is inert, never a mis-tint of some other edge).
    for sel in [None, Some("gone")] {
        let plain = connection_lane_verts(&segs, sel);
        assert!(plain.chunks_exact(6).all(|c| c[2..6] == CONN_LINE_RGBA[..]));
    }
    assert!(connection_lane_verts(&[], None).is_empty());
}

/// The hit test is point-to-SEGMENT: near the span hits, past the end does not (the
/// infinite-line form would select a short edge from far off its extension), and the NEAREST
/// edge wins when two are in range.
#[test]
fn pick_is_nearest_segment_within_tolerance() {
    let segs = connection_segments(&rows(&[("k1", "a", "b"), ("k2", "a", "c")]), &positions());
    // 3 m off the middle of the a→b edge, tolerance 5 m.
    assert_eq!(
        pick_connection(&segs, 50.0, 3.0, 5.0).as_deref(),
        Some("k1")
    );
    // Same perpendicular offset, outside tolerance.
    assert_eq!(pick_connection(&segs, 50.0, 9.0, 5.0), None);
    // On the a→b LINE but 40 m past its `b` end — the extension is not the edge.
    assert_eq!(pick_connection(&segs, 140.0, 0.0, 5.0), None);
    // Near the shared corner both edges are in range; the closer one wins.
    assert_eq!(
        pick_connection(&segs, 2.0, 9.0, 20.0).as_deref(),
        Some("k2")
    );
    assert_eq!(
        pick_connection(&segs, 9.0, 2.0, 20.0).as_deref(),
        Some("k1")
    );
    assert_eq!(pick_connection(&[], 0.0, 0.0, 100.0), None);
}
