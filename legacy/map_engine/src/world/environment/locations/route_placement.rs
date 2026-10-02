//! Role: route placement.
//! Position: `world/environment/locations` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::environment::locations::route_geometry::dist_m;
use crate::world::environment::locations::route_geometry::perpendicular_dist_to_polyline;
use crate::world::environment::locations::route_geometry::placement_fractions;
use crate::world::environment::locations::route_geometry::point_tangent_at_frac;
use crate::world::environment::locations::route_geometry::polyline_length;
use crate::world::environment::locations::route_geometry::road_declutter_min_dist_m;
use crate::world::environment::locations::route_geometry::upright_angle_deg;
use crate::world::environment::locations::route_labels::RoadNameEntry;
use crate::world::environment::locations::route_labels::RoadNamesFile;
use crate::world::terrain::roads::network::RoadSegment;

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

/// One candidate / drawn road label anchor in world meters.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RoadLabelPlacement {
    /// Name.
    pub name: String,

    /// X.
    pub x: f64,

    /// Y.
    pub y: f64,

    /// Screen CCW degrees for glyph yaw (upright tangent).
    pub angle_deg: f64,

    /// Priority.
    pub priority: u16,

    /// Segment id.
    #[serde(rename = "segmentId")]
    pub segment_id: String,

    /// Road class.
    #[serde(rename = "roadClass")]
    pub road_class: String,

    /// Arc frac.
    #[serde(rename = "arcFrac")]
    pub arc_frac: f64,
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

/// Left normal.
#[inline]
pub(crate) fn left_normal(tangent: [f64; 2]) -> [f64; 2] {
    [-tangent[1], tangent[0]]
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

/// Declutter + cap at `deck_zoom`.
#[must_use]
pub fn declutter_road_labels(
    candidates: &[RoadLabelPlacement],
    deck_zoom: f64,
) -> Vec<RoadLabelPlacement> {
    let mut sorted: Vec<RoadLabelPlacement> = candidates.to_vec();
    sorted.sort_by(road_declutter_order);
    declutter_road_labels_in_order(&sorted, deck_zoom)
}

/// Road declutter order.
pub(crate) fn road_declutter_order(
    a: &RoadLabelPlacement,
    b: &RoadLabelPlacement,
) -> std::cmp::Ordering {
    b.priority
        .cmp(&a.priority)
        .then_with(|| a.name.cmp(&b.name))
        .then_with(|| a.segment_id.cmp(&b.segment_id))
}

/// The greedy half of [`declutter_road_labels`], for candidates **already** in [`road_declutter_order`].
#[must_use]
pub fn declutter_road_labels_in_order(
    sorted: &[RoadLabelPlacement],
    deck_zoom: f64,
) -> Vec<RoadLabelPlacement> {
    let d_min = road_declutter_min_dist_m(deck_zoom);
    let mut keep: Vec<RoadLabelPlacement> = Vec::new();
    for cand in sorted {
        if keep.len() >= ROAD_NAME_MAX_ON_SCREEN {
            break;
        }
        let blocked = keep.iter().any(|k| dist_m(cand, k) < d_min);
        if !blocked {
            keep.push(cand.clone());
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

/// The closed road-class table, in `roads::road_style_width` order. The index **+1** is the `road_class` byte on the wire; `0` is "unknown class", which no gate admits.
pub const ROAD_CLASSES: [&str; 6] = [
    "highway_paved",
    "road_paved",
    "road_dirt",
    "track",
    "path",
    "runway",
];

/// Wire code for a road class (`0` when the class is not in [`ROAD_CLASSES`]).
#[must_use]
pub fn road_class_code(road_class: &str) -> u8 {
    ROAD_CLASSES
        .iter()
        .position(|c| *c == road_class)
        .map_or(0, |i| (i + 1) as u8)
}

/// Class name for a wire code (`""` for `0` / out of range).
#[must_use]
pub fn road_class_name(code: u8) -> &'static str {
    match usize::from(code)
        .checked_sub(1)
        .and_then(|i| ROAD_CLASSES.get(i))
    {
        Some(c) => c,
        None => "",
    }
}

/// The `deck_zoom` floor at which [`road_name_visible_for_class`] starts admitting `road_class`, or `None` when the class is never drawn.
#[must_use]
pub fn road_class_visibility_floor(road_class: &str) -> Option<f64> {
    if road_class == "highway_paved" || road_class == "runway" {
        Some(ROAD_NAME_MIN_ZOOM_HIGHWAY)
    } else if road_class == "road_paved" {
        Some(ROAD_NAME_MIN_ZOOM_SECONDARY)
    } else {
        None
    }
}
