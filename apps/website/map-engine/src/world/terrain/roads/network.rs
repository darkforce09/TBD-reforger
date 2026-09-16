//! Role: network.
//! Position: `world/terrain/roads` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use rkyv::Archived;
use serde_json::Value;

use crate::io::archives::codec::BinaryError;
use crate::io::archives::codec::access_checked;
use crate::io::archives::roads::RoadNetworkArchive;
use crate::io::archives::version::ARCHIVE_SCHEMA_VERSION;
use crate::world::environment::locations::route_placement::road_class_name;

/// Consecutive midpoints closer than this (m) are collapsed duplicate cross-edges.
pub const CENTERLINE_DEDUPE_M: f64 = 0.05;

/// Fallback style width (m) per road class — the closed `ROAD_STYLES` enum (`roadLayer.ts:39`). `None` for a class not in the table (segment dropped, matching `roadClass in ROAD_STYLES`).
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
    /// Id.
    pub id: String,

    /// Road class.
    pub road_class: String,

    /// Points.
    pub points: Vec<[f64; 2]>,

    /// Width m.
    pub width_m: f64,
}

/// `extractRoadCenterline(points)` (`roadLayer.ts:72`). Midpoint of each cross pair = centerline vertex; median cross-edge length = width. `None` when < 2 distinct midpoints.
#[must_use]
pub fn extract_road_centerline(points: &[[f64; 2]]) -> Option<(Vec<[f64; 2]>, f64)> {
    let mut path: Vec<[f64; 2]> = Vec::new();
    let mut widths: Vec<f64> = Vec::new();
    let pair_count = points.len() / 2;
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

/// `parseRoadsPayload(raw)` (`:95`). Keeps a segment iff `id` is a string, `roadClass` is a string in `ROAD_STYLES`, and `points` is `len ≥ 2` of finite pairs that centerline to ≥ 2 vertices. Width is the measured centerline width sanity-clamped to `(0.3, 40)`, else the style fallback.
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
            continue;
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

/// Canonical road network align value.
pub const ROAD_NETWORK_ALIGN: usize = 16;

const _: () = assert!(
    ROAD_NETWORK_ALIGN.is_multiple_of(align_of::<Archived<RoadNetworkArchive>>()),
    "ROAD_NETWORK_ALIGN must be a multiple of the archived type's own alignment"
);

/// `RoadNetworkArchive` → the very network [`parse_roads_payload`] yields from `roads.json.gz`.
pub fn from_archive(
    archive: &Archived<RoadNetworkArchive>,
) -> Result<Vec<RoadSegment>, BinaryError> {
    let mut out = Vec::with_capacity(archive.segments.len());
    for (i, s) in archive.segments.iter().enumerate() {
        let road_class = road_class_name(s.road_class);
        if road_class.is_empty() {
            return Err(BinaryError::Archive {
                what: "RoadNetworkArchive",
                cause: format!(
                    "segment {i} (id {:?}) carries road_class code {}, which this build's class \
                     table cannot name — the archive was written against a different table",
                    s.id.as_str(),
                    s.road_class
                ),
            });
        }
        out.push(RoadSegment {
            id: s.id.to_string(),
            road_class: road_class.to_string(),
            points: s
                .centerline
                .iter()
                .map(|p| [f64::from(p[0].to_native()), f64::from(p[1].to_native())])
                .collect(),
            width_m: f64::from(s.width_m.to_native()),
        });
    }
    Ok(out)
}

/// One `roads/road_network.rkyv` file → the in-memory road network, zero-copy and JSON-free.
pub fn road_network_from_bytes(raw: &[u8]) -> Result<Vec<RoadSegment>, BinaryError> {
    let (buf, pad) = aligned_copy(raw).ok_or(BinaryError::Misaligned {
        what: "RoadNetworkArchive",
        align: ROAD_NETWORK_ALIGN,
    })?;
    let archive = access_checked::<RoadNetworkArchive>(&buf[pad..])?;
    let version = archive.schema_version.to_native();
    if version != ARCHIVE_SCHEMA_VERSION {
        return Err(BinaryError::UnsupportedVersion {
            what: "RoadNetworkArchive",
            expected: ARCHIVE_SCHEMA_VERSION,
            actual: version,
        });
    }
    from_archive(archive)
}

fn aligned_copy(raw: &[u8]) -> Option<(Vec<u8>, usize)> {
    let mut buf: Vec<u8> = Vec::with_capacity(raw.len() + ROAD_NETWORK_ALIGN);
    let pad = buf.as_ptr().align_offset(ROAD_NETWORK_ALIGN);
    if pad >= ROAD_NETWORK_ALIGN {
        return None;
    }
    buf.resize(pad, 0);
    buf.extend_from_slice(raw);
    (buf[pad..].as_ptr().align_offset(ROAD_NETWORK_ALIGN) == 0).then_some((buf, pad))
}

#[cfg(test)]
#[path = "tests/network_tests.rs"]
mod tests;
