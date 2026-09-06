//! Road centerline extraction + payload narrowing — ports of `extractRoadCenterline` and
//! `parseRoadsPayload` (`roadLayer.ts:72`/`:95`). The export ships road-surface *quad soup*
//! (alternating cross-edge point pairs); the centerline midpoints each pair and measures the
//! median cross-edge width. Centerline vertices/width are **Class T** (≤ 1 ULP vs the TS).

use rkyv::Archived;
use serde_json::Value;

use super::binary::archives::{ARCHIVE_SCHEMA_VERSION, RoadNetworkArchive};
use super::binary::{BinaryError, access_checked};
use super::road_labels::road_class_name;

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

/* ─────────────────────── T-935.6 — the rkyv road network ─────────────────────── */

/// The alignment a `roads/road_network.rkyv` buffer must sit on before [`access_checked`] will
/// look at it.
///
/// 16 is what [`to_bytes`](super::binary::to_bytes)' `AlignedVec` writes with, and it is a multiple
/// of every alignment the archived type itself asks for — the `const` below is the proof, checked
/// at compile time rather than trusted. (Same reasoning, same number, as
/// [`MAP_LABELS_ALIGN`](crate::world::MAP_LABELS_ALIGN); they are separate consts because they are
/// separate formats and either type's layout may move independently.)
pub const ROAD_NETWORK_ALIGN: usize = 16;

const _: () = assert!(
    ROAD_NETWORK_ALIGN.is_multiple_of(align_of::<Archived<RoadNetworkArchive>>()),
    "ROAD_NETWORK_ALIGN must be a multiple of the archived type's own alignment"
);

/// `RoadNetworkArchive` → the very network [`parse_roads_payload`] yields from `roads.json.gz`.
///
/// # This is the f32 projection, and that is the whole contract
///
/// [`RoadSegment`] is `f64` because it mirrors JSON numbers under a Class-R parity contract; the
/// wire row is `f32` because a centreline vertex ends up in an `f32` vertex buffer either way
/// (`archives.rs` module docs). So the round trip is **not** the identity on `f64` — it is
/// `f64::from(v as f32)`, and the parity pin states it that way rather than comparing `==` and
/// hoping. Nothing here re-derives a centreline: the emitter ran [`extract_road_centerline`] once
/// at build time, which is the point of the format.
///
/// # Errors
/// [`BinaryError::Archive`] when a segment carries a `road_class` code this build's class table
/// cannot name. That is deliberately fatal rather than a dropped or empty-classed segment: an
/// unnameable code means the writer and the reader disagree about what the byte *means*, and a
/// segment with class `""` matches no entry in `road_style`, so it would vanish from the map
/// silently and permanently — the exact failure mode a shifted enum is supposed to announce.
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
///
/// The buffer is copied once into an aligned one because `fs::read`/`fetch` hand back a `Vec<u8>`
/// that is only 1-aligned by contract; after that nothing is deserialised — [`from_archive`] reads
/// the validated archive in place.
///
/// # Errors
/// * [`BinaryError::Misaligned`] — the aligned copy could not be placed (the allocator refused to
///   say where it put the buffer). Recoverable by falling back to the JSON path.
/// * [`BinaryError::Archive`] — rkyv validation rejected the buffer (truncated, corrupt, not this
///   format), or a segment's class code is unnameable (see [`from_archive`]).
/// * [`BinaryError::UnsupportedVersion`] — a well-formed archive written by a different schema.
///   `access_checked` cannot catch this: the layout is legal, the *meaning* is not.
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

/// `raw` copied into a buffer whose byte at `pad` sits on [`ROAD_NETWORK_ALIGN`].
///
/// Capacity is reserved up front so the `extend_from_slice` cannot reallocate and move the
/// alignment out from under the offset that was just measured; the result is re-checked anyway.
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

    /* ───────────────────── T-935.6 — the archive reader ───────────────────── */

    use super::super::binary::archives::RoadSegmentArchive;
    use super::super::binary::to_bytes;
    use super::super::road_labels::road_class_code;

    fn network(segments: Vec<RoadSegmentArchive>) -> RoadNetworkArchive {
        RoadNetworkArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            segments,
        }
    }

    fn one_segment() -> RoadNetworkArchive {
        network(vec![RoadSegmentArchive {
            id: "road-everon-0007".to_string(),
            road_class: road_class_code("road_dirt"),
            width_m: 2.5,
            centerline: vec![[1.5, -2.25], [10.0, -2.25], [10.0, 40.5]],
        }])
    }

    /// The reader's happy path, end to end over real bytes: serialise, read back through the
    /// validating entry point, and get the parser's own struct out with every field intact.
    #[test]
    fn road_network_round_trips_through_the_validating_reader() {
        let bytes = to_bytes(&one_segment()).expect("serialise");
        let segs = road_network_from_bytes(&bytes).expect("read back");
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].id, "road-everon-0007");
        assert_eq!(segs[0].road_class, "road_dirt");
        assert_eq!(segs[0].width_m, 2.5);
        assert_eq!(
            segs[0].points,
            vec![[1.5, -2.25], [10.0, -2.25], [10.0, 40.5]]
        );
    }

    /// The archive is the `f32` projection of the JSON network, so the contract is
    /// `f64::from(v as f32)` — not `==` on the `f64`. Pin a value that is NOT f32-exact so the
    /// statement is load-bearing: `0.1_f64 as f32` widens back to `0.100000001490116...`.
    #[test]
    fn archive_widens_f32_exactly_and_does_not_pretend_to_be_f64() {
        let bytes = to_bytes(&network(vec![RoadSegmentArchive {
            id: "r".to_string(),
            road_class: road_class_code("track"),
            width_m: 0.1,
            centerline: vec![[0.1, 0.2], [0.3, 0.4]],
        }]))
        .expect("serialise");
        let segs = road_network_from_bytes(&bytes).expect("read back");
        assert_eq!(segs[0].width_m, f64::from(0.1_f32));
        assert_ne!(segs[0].width_m, 0.1_f64, "0.1 is not f32-exact");
        assert_eq!(segs[0].points[0], [f64::from(0.1_f32), f64::from(0.2_f32)]);
    }

    /// A class code this build cannot name is an error, never a segment with class `""` — which
    /// `road_style` would reject, deleting the road from the map with nothing said.
    #[test]
    fn unnameable_class_code_is_an_error_not_a_vanished_road() {
        for code in [0u8, 7, 255] {
            let bytes = to_bytes(&network(vec![RoadSegmentArchive {
                id: "r0".to_string(),
                road_class: code,
                width_m: 4.0,
                centerline: vec![[0.0, 0.0], [1.0, 1.0]],
            }]))
            .expect("serialise");
            let err = road_network_from_bytes(&bytes).expect_err("must refuse");
            let msg = err.to_string();
            assert!(msg.contains("cannot name"), "{code}: {msg}");
            assert!(msg.contains("road_class code"), "{code}: {msg}");
        }
        // …and every code the table DOES name round-trips, so the guard is not simply "always red".
        for class in super::super::road_labels::ROAD_CLASSES {
            let bytes = to_bytes(&network(vec![RoadSegmentArchive {
                id: "r0".to_string(),
                road_class: road_class_code(class),
                width_m: 4.0,
                centerline: vec![[0.0, 0.0], [1.0, 1.0]],
            }]))
            .expect("serialise");
            let segs = road_network_from_bytes(&bytes).unwrap_or_else(|e| panic!("{class}: {e}"));
            assert_eq!(segs[0].road_class, class);
        }
    }

    /// A legal layout written by a different schema is `UnsupportedVersion`, not a silent read.
    /// `access_checked` cannot catch this — the bytes validate; the *meaning* is what moved.
    #[test]
    fn wrong_schema_version_is_refused_even_though_the_bytes_validate() {
        let mut archive = one_segment();
        archive.schema_version = ARCHIVE_SCHEMA_VERSION + 1;
        let bytes = to_bytes(&archive).expect("serialise");
        assert!(
            access_checked::<RoadNetworkArchive>(&bytes).is_ok(),
            "the buffer must be structurally valid, or this test proves nothing"
        );
        assert!(matches!(
            road_network_from_bytes(&bytes),
            Err(BinaryError::UnsupportedVersion {
                what: "RoadNetworkArchive",
                expected: ARCHIVE_SCHEMA_VERSION,
                actual,
            }) if actual == ARCHIVE_SCHEMA_VERSION + 1
        ));
    }

    /// Truncated, empty, and plain-JSON buffers all land in an error rather than a wrong answer.
    #[test]
    fn corrupt_buffers_are_refused() {
        let bytes = to_bytes(&one_segment()).expect("serialise");
        assert!(road_network_from_bytes(&[]).is_err(), "empty");
        assert!(
            road_network_from_bytes(&bytes[..bytes.len() - 1]).is_err(),
            "truncated tail"
        );
        assert!(
            road_network_from_bytes(&bytes[1..]).is_err(),
            "truncated head"
        );
        assert!(
            road_network_from_bytes(br#"{"roadSegments":[]}"#).is_err(),
            "JSON must not be read as an archive"
        );
    }
}
