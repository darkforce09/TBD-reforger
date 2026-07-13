//! Polyline-following road name labels + declutter (T-152.9).
//! Curated `road-names.json` joined to centerlined [`RoadSegment`] geometry.

#![forbid(unsafe_code)]

use super::roads::RoadSegment;

/// Label-label minimum separation at `deck_zoom = 0` (spec L5 / G6).
pub const ROAD_NAME_DECLUTTER_BASE_M: f64 = 60.0;
/// Normal offset from centerline (m).
pub const ROAD_NAME_OFFSET_M: f64 = 6.0;
/// Extra placement fractions when polyline length exceeds this (m).
pub const ROAD_NAME_LONG_SEGMENT_M: f64 = 3000.0;
/// Max labels after declutter (spec L5 / G7).
pub const ROAD_NAME_MAX_ON_SCREEN: usize = 24;
/// Highway names visible from this deck zoom (spec §Goal.6).
pub const ROAD_NAME_MIN_ZOOM_HIGHWAY: f64 = 0.0;
/// Secondary paved names from this deck zoom.
pub const ROAD_NAME_MIN_ZOOM_SECONDARY: f64 = 1.0;
/// G5 perpendicular tolerance (m).
pub const ROAD_NAME_PERP_TOL_M: f64 = 12.0;

/// One curated named route from `road-names.json`.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RoadNameEntry {
    pub id: String,
    pub name: String,
    #[serde(rename = "segmentIds")]
    pub segment_ids: Vec<String>,
    /// Optional visibility floor (curated major routes pin `0` per G3 @ z=0).
    #[serde(rename = "minDeckZoom", default)]
    pub min_deck_zoom: Option<f64>,
}

/// Payload root for `road-names.json`.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RoadNamesFile {
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    #[serde(rename = "terrainId")]
    pub terrain_id: String,
    pub roads: Vec<RoadNameEntry>,
}

/// One candidate / drawn road label anchor in world meters.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RoadLabelPlacement {
    pub name: String,
    pub x: f64,
    pub y: f64,
    /// Screen CCW degrees for glyph yaw (upright tangent).
    pub angle_deg: f64,
    pub priority: u16,
    #[serde(rename = "segmentId")]
    pub segment_id: String,
    #[serde(rename = "roadClass")]
    pub road_class: String,
    #[serde(rename = "arcFrac")]
    pub arc_frac: f64,
}

/// Parse `road-names.json`.
///
/// # Errors
/// Returns a message when JSON is invalid.
pub fn parse_road_names_json(json: &str) -> Result<RoadNamesFile, String> {
    serde_json::from_str(json).map_err(|e| format!("road-names json: {e}"))
}

/// Declutter threshold in world meters at `deck_zoom`.
#[must_use]
pub fn road_declutter_min_dist_m(deck_zoom: f64) -> f64 {
    ROAD_NAME_DECLUTTER_BASE_M * 2f64.powf(-deck_zoom)
}

/// Priority for road-class collision resolution (higher wins).
#[must_use]
pub fn road_class_priority(road_class: &str) -> u16 {
    match road_class {
        "highway_paved" => 400,
        "road_paved" => 300,
        "road_dirt" => 200,
        "track" => 100,
        "path" => 50,
        "runway" => 350,
        _ => 0,
    }
}

/// Whether a curated entry is visible at `deck_zoom` (entry override beats class gate).
#[must_use]
pub fn road_entry_visible(entry: &RoadNameEntry, road_class: &str, deck_zoom: f64) -> bool {
    if let Some(min) = entry.min_deck_zoom {
        deck_zoom >= min
    } else {
        road_name_visible_for_class(road_class, deck_zoom)
    }
}

/// Whether a segment class is visible at `deck_zoom`.
#[must_use]
pub fn road_name_visible_for_class(road_class: &str, deck_zoom: f64) -> bool {
    if road_class == "highway_paved" || road_class == "runway" {
        deck_zoom >= ROAD_NAME_MIN_ZOOM_HIGHWAY
    } else if road_class == "road_paved" {
        deck_zoom >= ROAD_NAME_MIN_ZOOM_SECONDARY
    } else {
        false
    }
}

/// Total polyline arc length (m).
#[must_use]
pub fn polyline_length(points: &[[f64; 2]]) -> f64 {
    if points.len() < 2 {
        return 0.0;
    }
    let mut len = 0.0;
    for w in points.windows(2) {
        let dx = w[1][0] - w[0][0];
        let dy = w[1][1] - w[0][1];
        len += dx.hypot(dy);
    }
    len
}

/// Point + unit tangent at normalized arc fraction `frac` ∈ [0,1].
#[must_use]
pub fn point_tangent_at_frac(points: &[[f64; 2]], frac: f64) -> Option<([f64; 2], [f64; 2])> {
    if points.len() < 2 {
        return None;
    }
    let total = polyline_length(points);
    if total < 1e-6 {
        return None;
    }
    let target = frac.clamp(0.0, 1.0) * total;
    let mut acc = 0.0;
    for w in points.windows(2) {
        let a = w[0];
        let b = w[1];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let seg_len = dx.hypot(dy);
        if seg_len < 1e-12 {
            continue;
        }
        if acc + seg_len >= target {
            let t = ((target - acc) / seg_len).clamp(0.0, 1.0);
            let px = a[0] + dx * t;
            let py = a[1] + dy * t;
            let tx = dx / seg_len;
            let ty = dy / seg_len;
            return Some(([px, py], [tx, ty]));
        }
        acc += seg_len;
    }
    let a = points[points.len() - 2];
    let b = points[points.len() - 1];
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let seg_len = dx.hypot(dy).max(1e-12);
    Some(([b[0], b[1]], [dx / seg_len, dy / seg_len]))
}

/// Upright screen CCW angle from unit tangent (spec L4).
#[must_use]
pub fn upright_angle_deg(tangent: [f64; 2]) -> f64 {
    let mut deg = tangent[1].atan2(tangent[0]).to_degrees();
    if deg.abs() > 90.0 {
        deg += 180.0;
    }
    if deg > 180.0 {
        deg -= 360.0;
    }
    if deg <= -180.0 {
        deg += 360.0;
    }
    deg
}

/// Left normal of unit tangent.
#[inline]
fn left_normal(tangent: [f64; 2]) -> [f64; 2] {
    [-tangent[1], tangent[0]]
}

/// Placement fractions for a segment length.
#[must_use]
pub fn placement_fractions(length_m: f64) -> Vec<f64> {
    if length_m > ROAD_NAME_LONG_SEGMENT_M {
        vec![0.25, 0.5, 0.75]
    } else {
        vec![0.5]
    }
}

/// Perpendicular distance from `(px,py)` to polyline (m).
#[must_use]
pub fn perpendicular_dist_to_polyline(points: &[[f64; 2]], px: f64, py: f64) -> f64 {
    if points.len() < 2 {
        return f64::INFINITY;
    }
    let mut best = f64::INFINITY;
    for w in points.windows(2) {
        let ax = w[0][0];
        let ay = w[0][1];
        let bx = w[1][0];
        let by = w[1][1];
        let dx = bx - ax;
        let dy = by - ay;
        let len_sq = dx * dx + dy * dy;
        if len_sq < 1e-18 {
            let d = (px - ax).hypot(py - ay);
            if d < best {
                best = d;
            }
            continue;
        }
        let t = ((px - ax) * dx + (py - ay) * dy) / len_sq;
        let t = t.clamp(0.0, 1.0);
        let cx = ax + dx * t;
        let cy = ay + dy * t;
        let d = (px - cx).hypot(py - cy);
        if d < best {
            best = d;
        }
    }
    best
}

/// Build candidate placements from curated names + parsed segments.
#[must_use]
pub fn place_road_labels(
    names: &RoadNamesFile,
    segments: &[RoadSegment],
    deck_zoom: f64,
) -> Vec<RoadLabelPlacement> {
    let by_id: std::collections::HashMap<&str, &RoadSegment> =
        segments.iter().map(|s| (s.id.as_str(), s)).collect();
    let mut out = Vec::new();
    for entry in &names.roads {
        let name = entry.name.trim();
        if name.len() < 2 {
            continue;
        }
        for seg_id in &entry.segment_ids {
            let Some(seg) = by_id.get(seg_id.as_str()) else {
                continue;
            };
            if !road_entry_visible(entry, &seg.road_class, deck_zoom) {
                continue;
            }
            let len = polyline_length(&seg.points);
            if len < 1.0 {
                continue;
            }
            for frac in placement_fractions(len) {
                let Some((pt, tan)) = point_tangent_at_frac(&seg.points, frac) else {
                    continue;
                };
                let n = left_normal(tan);
                let x = pt[0] + n[0] * ROAD_NAME_OFFSET_M;
                let y = pt[1] + n[1] * ROAD_NAME_OFFSET_M;
                out.push(RoadLabelPlacement {
                    name: name.to_string(),
                    x,
                    y,
                    angle_deg: upright_angle_deg(tan),
                    priority: road_class_priority(&seg.road_class)
                        + if entry.min_deck_zoom.is_some() { 80 } else { 0 },
                    segment_id: seg_id.clone(),
                    road_class: seg.road_class.clone(),
                    arc_frac: frac,
                });
            }
        }
    }
    out
}

fn dist_m(a: &RoadLabelPlacement, b: &RoadLabelPlacement) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// Declutter + cap at `deck_zoom`.
#[must_use]
pub fn declutter_road_labels(
    candidates: &[RoadLabelPlacement],
    deck_zoom: f64,
) -> Vec<RoadLabelPlacement> {
    let d_min = road_declutter_min_dist_m(deck_zoom);
    let mut sorted: Vec<RoadLabelPlacement> = candidates.to_vec();
    sorted.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.segment_id.cmp(&b.segment_id))
    });
    let mut keep: Vec<RoadLabelPlacement> = Vec::new();
    for cand in sorted {
        if keep.len() >= ROAD_NAME_MAX_ON_SCREEN {
            break;
        }
        let blocked = keep.iter().any(|k| dist_m(&cand, k) < d_min);
        if !blocked {
            keep.push(cand);
        }
    }
    keep
}

/// Full pipeline: place then declutter.
#[must_use]
pub fn build_road_label_draw_set(
    names: &RoadNamesFile,
    segments: &[RoadSegment],
    deck_zoom: f64,
) -> Vec<RoadLabelPlacement> {
    let candidates = place_road_labels(names, segments, deck_zoom);
    declutter_road_labels(&candidates, deck_zoom)
}

/// G6: every pair in `drawn` is ≥ d_min apart.
#[must_use]
pub fn road_declutter_invariant_holds(drawn: &[RoadLabelPlacement], deck_zoom: f64) -> bool {
    let d_min = road_declutter_min_dist_m(deck_zoom);
    for (i, a) in drawn.iter().enumerate() {
        for b in drawn.iter().skip(i + 1) {
            if dist_m(a, b) < d_min {
                return false;
            }
        }
    }
    true
}

/// G5: every label within perpendicular tolerance of its segment polyline.
#[must_use]
pub fn road_placement_geometry_holds(
    drawn: &[RoadLabelPlacement],
    segments: &[RoadSegment],
) -> bool {
    let by_id: std::collections::HashMap<&str, &RoadSegment> =
        segments.iter().map(|s| (s.id.as_str(), s)).collect();
    drawn.iter().all(|lab| {
        by_id
            .get(lab.segment_id.as_str())
            .map(|seg| {
                perpendicular_dist_to_polyline(&seg.points, lab.x, lab.y) <= ROAD_NAME_PERP_TOL_M
            })
            .unwrap_or(false)
    })
}

/// G4: every name has length ≥ 2.
#[must_use]
pub fn road_name_schema_holds(drawn: &[RoadLabelPlacement]) -> bool {
    drawn.iter().all(|l| l.name.trim().len() >= 2)
}

/// G3 fuzzy match: each required name appears in drawn set.
#[must_use]
pub fn major_roads_covered(drawn: &[RoadLabelPlacement], required: &[&str]) -> bool {
    let norm = |s: &str| s.to_lowercase().replace([' ', '-'], "");
    let drawn_norm: Vec<String> = drawn.iter().map(|l| norm(&l.name)).collect();
    required.iter().all(|req| {
        let k = norm(req);
        drawn_norm
            .iter()
            .any(|n| n == &k || n.contains(&k) || k.contains(n))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn seg(id: &str, cls: &str, points: Vec<[f64; 2]>) -> RoadSegment {
        RoadSegment {
            id: id.into(),
            road_class: cls.into(),
            points,
            width_m: 4.0,
        }
    }

    #[test]
    fn declutter_dist_scales_with_zoom() {
        let d0 = road_declutter_min_dist_m(0.0);
        let d1 = road_declutter_min_dist_m(1.0);
        assert!((d0 - 60.0).abs() < 1e-6);
        assert!((d1 - 30.0).abs() < 1e-6);
    }

    #[test]
    fn upright_flips_past_90() {
        let down = upright_angle_deg([0.0, -1.0]);
        assert!(down.abs() <= 90.0);
        let left = upright_angle_deg([-1.0, 0.0]);
        assert!(left.abs() <= 90.0);
    }

    #[test]
    fn long_segment_gets_three_fractions() {
        let fr = placement_fractions(4000.0);
        assert_eq!(fr, vec![0.25, 0.5, 0.75]);
        assert_eq!(placement_fractions(1000.0), vec![0.5]);
    }

    #[test]
    fn placement_within_perp_tol() {
        let points = vec![[0.0, 0.0], [1000.0, 0.0]];
        let s = seg("r1", "highway_paved", points.clone());
        let names = RoadNamesFile {
            schema_version: "1".into(),
            terrain_id: "everon".into(),
            roads: vec![RoadNameEntry {
                id: "t".into(),
                name: "Test Highway".into(),
                segment_ids: vec!["r1".into()],
                min_deck_zoom: None,
            }],
        };
        let drawn = build_road_label_draw_set(&names, std::slice::from_ref(&s), 0.0);
        assert_eq!(drawn.len(), 1);
        assert!(road_placement_geometry_holds(&drawn, &[s]));
    }

    #[test]
    fn close_labels_drop_lower_priority() {
        let a = RoadLabelPlacement {
            name: "A".into(),
            x: 0.0,
            y: 0.0,
            angle_deg: 0.0,
            priority: 400,
            segment_id: "a".into(),
            road_class: "highway_paved".into(),
            arc_frac: 0.5,
        };
        let b = RoadLabelPlacement {
            name: "B".into(),
            x: 10.0,
            y: 0.0,
            angle_deg: 0.0,
            priority: 100,
            segment_id: "b".into(),
            road_class: "track".into(),
            arc_frac: 0.5,
        };
        let out = declutter_road_labels(&[b, a], 0.0);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "A");
        assert!(road_declutter_invariant_holds(&out, 0.0));
    }

    #[test]
    fn cap_at_24() {
        let mut cands = Vec::new();
        for i in 0..40 {
            cands.push(RoadLabelPlacement {
                name: format!("Road {i}"),
                x: f64::from(i) * 100.0,
                y: 0.0,
                angle_deg: 0.0,
                priority: 300,
                segment_id: format!("s{i}"),
                road_class: "road_paved".into(),
                arc_frac: 0.5,
            });
        }
        let out = declutter_road_labels(&cands, 0.0);
        assert!(out.len() <= ROAD_NAME_MAX_ON_SCREEN);
    }

    #[test]
    fn parse_road_names_sample() {
        let json = json!({
            "schemaVersion": "1.0.0",
            "terrainId": "everon",
            "roads": [{ "id": "x", "name": "Main Highway", "segmentIds": ["road-everon-0010"] }]
        });
        let file = parse_road_names_json(&json.to_string()).expect("parse");
        assert_eq!(file.roads[0].name, "Main Highway");
    }
}
