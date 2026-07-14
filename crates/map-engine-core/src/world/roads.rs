//! Road centerline extraction + payload narrowing — ports of `extractRoadCenterline` and
//! `parseRoadsPayload` (`roadLayer.ts:72`/`:95`). The export ships road-surface *quad soup*
//! (alternating cross-edge point pairs); the centerline midpoints each pair and measures the
//! median cross-edge width. Centerline vertices/width are **Class T** (≤ 1 ULP vs the TS).

use serde_json::Value;

/// Consecutive midpoints closer than this (m) are collapsed duplicate cross-edges.
pub const CENTERLINE_DEDUPE_M: f64 = 0.05;

/// Fallback style width (m) per road class — the closed `ROAD_STYLES` enum (`roadLayer.ts:39`).
/// `None` for a class not in the table (segment dropped, matching `roadClass in ROAD_STYLES`).
#[must_use]
pub fn road_style_width(road_class: &str) -> Option<f64> {
    Some(match road_class {
        "highway_paved" => 4.0,
        "road_paved" => 2.5,
        "road_dirt" => 2.0,
        "track" => 1.5,
        "path" => 1.0,
        "runway" => 20.0,
        _ => return None,
    })
}

/// One centerlined road (mirror of `RoadSegment`). `points` are centerline vertices, y-up.
#[derive(Clone, Debug, PartialEq)]
pub struct RoadSegment {
    pub id: String,
    pub road_class: String,
    pub points: Vec<[f64; 2]>,
    pub width_m: f64,
}

/// `extractRoadCenterline(points)` (`roadLayer.ts:72`). Midpoint of each cross pair =
/// centerline vertex; median cross-edge length = width. `None` when < 2 distinct midpoints.
///
/// Bit-exact: midpoint `(a + b) / 2` (not `a + (b-a)/2`); dedupe distance and cross-edge width
/// via `hypot`; the odd trailing point is dropped; width = `sorted_ascending[len / 2]`.
#[must_use]
pub fn extract_road_centerline(points: &[[f64; 2]]) -> Option<(Vec<[f64; 2]>, f64)> {
    let mut path: Vec<[f64; 2]> = Vec::new();
    let mut widths: Vec<f64> = Vec::new();
    let pair_count = points.len() / 2; // floor; odd trailing point dropped
    for k in 0..pair_count {
        let a = points[2 * k];
        let b = points[2 * k + 1];
        let mx = (a[0] + b[0]) / 2.0;
        let my = (a[1] + b[1]) / 2.0;
        if let Some(prev) = path.last()
            && (mx - prev[0]).hypot(my - prev[1]) < CENTERLINE_DEDUPE_M
        {
            continue;
        }
        path.push([mx, my]);
        widths.push((b[0] - a[0]).hypot(b[1] - a[1]));
    }
    if path.len() < 2 {
        return None;
    }
    let mut sorted = widths.clone();
    sorted.sort_by(|x, y| x.partial_cmp(y).expect("road widths are finite"));
    let width_m = sorted[sorted.len() / 2];
    Some((path, width_m))
}

/// A road point in the payload: array of length ≥ 2 with finite `[0]`/`[1]` (`isPoint`; extra
/// components such as a z ignored).
#[must_use]
fn narrow_point(p: &Value) -> Option<[f64; 2]> {
    let a = p.as_array()?;
    if a.len() < 2 {
        return None;
    }
    let x = a[0].as_f64().filter(|n| n.is_finite())?;
    let y = a[1].as_f64().filter(|n| n.is_finite())?;
    Some([x, y])
}

/// `parseRoadsPayload(raw)` (`:95`). Keeps a segment iff `id` is a string, `roadClass` is a
/// string in `ROAD_STYLES`, and `points` is `len ≥ 2` of finite pairs that centerline to ≥ 2
/// vertices. Width is the measured centerline width sanity-clamped to `(0.3, 40)`, else the
/// style fallback.
#[must_use]
pub fn parse_roads_payload(raw: &Value) -> Vec<RoadSegment> {
    let Some(segments) = raw.get("roadSegments").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for s in segments {
        let Some(id) = s.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(road_class) = s.get("roadClass").and_then(Value::as_str) else {
            continue;
        };
        let Some(fallback) = road_style_width(road_class) else {
            continue; // class not in ROAD_STYLES
        };
        let Some(raw_points) = s.get("points").and_then(Value::as_array) else {
            continue;
        };
        if raw_points.len() < 2 {
            continue;
        }
        let mut points = Vec::with_capacity(raw_points.len());
        let mut all_ok = true;
        for p in raw_points {
            match narrow_point(p) {
                Some(pt) => points.push(pt),
                None => {
                    all_ok = false;
                    break;
                }
            }
        }
        if !all_ok {
            continue;
        }
        let Some((path, width)) = extract_road_centerline(&points) else {
            continue;
        };
        let width_m = if width > 0.3 && width < 40.0 {
            width
        } else {
            fallback
        };
        out.push(RoadSegment {
            id: id.to_string(),
            road_class: road_class.to_string(),
            points: path,
            width_m,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Road along +y at x=0, true width 4 m, in export quad-soup form (mirrors roadLayer.test.ts).
    const QUAD_SOUP: [[f64; 2]; 8] = [
        [-2.0, 0.0],
        [2.0, 0.0],
        [2.0, 10.0],
        [-2.0, 10.0],
        [-2.0, 10.0],
        [2.0, 10.0],
        [2.0, 20.0],
        [-2.0, 20.0],
    ];

    #[test]
    fn centerline_midpoints_dedupes_measures() {
        let (path, width) = extract_road_centerline(&QUAD_SOUP).unwrap();
        assert_eq!(path, vec![[0.0, 0.0], [0.0, 10.0], [0.0, 20.0]]);
        assert_eq!(width, 4.0);
    }

    #[test]
    fn centerline_drops_odd_trailing_point() {
        let mut pts = QUAD_SOUP.to_vec();
        pts.push([999.0, 999.0]);
        let (path, _) = extract_road_centerline(&pts).unwrap();
        assert_eq!(path, vec![[0.0, 0.0], [0.0, 10.0], [0.0, 20.0]]);
    }

    #[test]
    fn centerline_null_when_under_two_midpoints() {
        assert!(extract_road_centerline(&[[-2.0, 0.0], [2.0, 0.0]]).is_none());
        assert!(
            extract_road_centerline(&[[-2.0, 0.0], [2.0, 0.0], [2.0, 0.0], [-2.0, 0.0]]).is_none()
        );
        assert!(extract_road_centerline(&[]).is_none());
    }

    #[test]
    fn width_is_median_across_cross_edges() {
        // Third cross-edge is a 12 m flare; median of [4,4,12] = 4.
        let (_p, width) = extract_road_centerline(&[
            [-2.0, 0.0],
            [2.0, 0.0],
            [2.0, 10.0],
            [-2.0, 10.0],
            [-6.0, 20.0],
            [6.0, 20.0],
        ])
        .unwrap();
        assert_eq!(width, 4.0);
    }

    #[test]
    fn parse_payload_narrows_good_segments() {
        let raw = json!({
            "roadSegments": [
                { "id": "r0", "roadClass": "runway", "points": QUAD_SOUP },
                { "id": "r1", "roadClass": "road_dirt", "points": [[0, -1], [0, 1], [10, 1], [10, -1]] }
            ]
        });
        let segs = parse_roads_payload(&raw);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].points, vec![[0.0, 0.0], [0.0, 10.0], [0.0, 20.0]]);
        assert_eq!(segs[0].width_m, 4.0);
        assert_eq!(segs[1].points, vec![[0.0, 0.0], [10.0, 0.0]]);
        assert_eq!(segs[1].width_m, 2.0);
    }

    #[test]
    fn parse_payload_drops_malformed() {
        let raw = json!({
            "roadSegments": [
                { "id": "x", "roadClass": "hyperloop", "points": QUAD_SOUP },        // unknown class
                { "id": "y", "roadClass": "track", "points": [[0, 0]] },              // < 2 points
                { "id": "z", "roadClass": "track", "points": [[0, 0], [null, 1]] },   // non-finite
                { "roadClass": "track", "points": QUAD_SOUP },                        // no id
                { "id": "w", "roadClass": "track", "points": [[-2, 0], [2, 0]] }      // single cross-edge → no centerline
            ]
        });
        assert_eq!(parse_roads_payload(&raw).len(), 0);
        assert_eq!(parse_roads_payload(&Value::Null).len(), 0);
        assert_eq!(parse_roads_payload(&json!("<html>")).len(), 0);
    }
}
