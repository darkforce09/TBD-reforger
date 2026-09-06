//! Parse `locations.json` into [`LocationLabel`] rows (T-152.8), plus the T-935.7 bridge between
//! those rows / `height-labels.json` and the [`MapLabelsArchive`] wire types.
//!
//! # Why the archive bridge lives here and not in `binary::archives`
//!
//! `world::binary::archives` is deliberately parser-agnostic — it is layout only, with no
//! `serde_json` — so the JSON⇄archive conversion has to live on the parser side. Both directions
//! sit in one module so the lossy edges are stated once, next to the code that causes them:
//!
//! | JSON field | Wire field | Round trip |
//! |---|---|---|
//! | `LocationLabel::name` / `kind` | `TownLabel::name` / `kind` | exact (`kind: None` ⇄ `""`) |
//! | `LocationLabel::x` / `y` / `importance` (`f64`) | `TownLabel::position` / `importance` (`f32`) | quantised to `f32` |
//! | `LocationLabel::id` | — | **dropped**: the wire type has no id field |
//! | `HeightLabel::x` / `y` / `value_m` | `HeightLabel::position` / `elevation_m` | `f32`-quantised, `value_m` exact |
//! | `HeightLabel::name` / `kind` | — | **dropped**: the wire type carries position + elevation only |
//!
//! The `f32` quantisation costs nothing that renders: every consumer of these rows
//! (`locations_to_label_specs`, `map_engine_render::text_layout::pack_town_label_glyphs`,
//! `height_labels_to_specs`) casts to `f32` or rounds to an integer before packing, so the archive
//! carries the same bits the GPU lane would have received. The **dropped** rows are not free, and
//! the T-935.7 parity tests assert quantisation-exactness rather than pretend otherwise:
//! `LocationLabel::id` is read by no consumer of the label host (`world_assets::named_locations`
//! → `dock_left::load_named_places` reads name/x/y/kind; the render lane reads name/x/y/importance),
//! and `HeightLabel::name`/`kind` are `None`/`Peak` for every label the SPA renders, because the SPA
//! height lane is `find_peaks` over the DEM raster and never the named export.

#![forbid(unsafe_code)]

use crate::dem::peaks::{HeightLabel, HeightLabelKind};
use crate::label::LabelSpec;
use crate::world::binary::archives::{
    ARCHIVE_SCHEMA_VERSION, HeightLabel as HeightLabelWire, MapLabelsArchive, TownLabel,
};
use crate::world::binary::{BinaryError, access_checked};
use crate::world::importance_declutter::LocationLabel;
use crate::world::road_labels::{RoadLabelPlacement, road_names_from_archive};

use rkyv::Archived;

/// Parse a `locations.json` array payload.
///
/// # Errors
/// Returns a message when JSON is not an array of location objects.
pub fn parse_locations_json(json: &str) -> Result<Vec<LocationLabel>, String> {
    serde_json::from_str(json).map_err(|e| format!("locations json: {e}"))
}

/// Parse a `height-labels.json` array payload into [`HeightLabel`] rows.
///
/// Hand-rolled rather than `serde` because [`HeightLabel`] carries no derives: it is a `dem`
/// compute type that predates the file, and adding `Deserialize` to it would pull `serde` into
/// every `dem`-only build (the `dem` module is *not* feature-gated — `lib.rs:44`). The shape is the
/// exporter's own (`tbd_tools::map::labels::export_height_labels`): `{x, y, value_m, kind, name?}`.
///
/// # Errors
/// Returns a message when the payload is not an array, or a row is missing `x`/`y`/`value_m`.
pub fn parse_height_labels_json(json: &str) -> Result<Vec<HeightLabel>, String> {
    let raw: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("height-labels json: {e}"))?;
    let rows = raw
        .as_array()
        .ok_or_else(|| "height-labels json: payload is not an array".to_string())?;
    let mut out = Vec::with_capacity(rows.len());
    for (i, r) in rows.iter().enumerate() {
        let num = |k: &str| r.get(k).and_then(serde_json::Value::as_f64);
        let (Some(x), Some(y), Some(value_m)) = (num("x"), num("y"), num("value_m")) else {
            return Err(format!("height-labels json: row {i} missing x/y/value_m"));
        };
        out.push(HeightLabel {
            x,
            y,
            #[allow(clippy::cast_possible_truncation)]
            value_m: value_m.round() as i32,
            kind: match r.get("kind").and_then(serde_json::Value::as_str) {
                Some("contour") => HeightLabelKind::Contour,
                _ => HeightLabelKind::Peak,
            },
            name: r
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
        });
    }
    Ok(out)
}

/// Map locations → text [`LabelSpec`] for glyph packing (importance scaled to u16).
#[must_use]
pub fn locations_to_label_specs(locations: &[LocationLabel]) -> Vec<LabelSpec> {
    locations
        .iter()
        .enumerate()
        .map(|(i, loc)| LabelSpec {
            id: i as u32,
            x: loc.x.round() as i32,
            y: loc.y.round() as i32,
            importance: (loc.importance.clamp(0.0, 1.0) * 10_000.0).round() as u16,
            text: loc.name.trim().to_string(),
        })
        .filter(|s| !s.text.is_empty())
        .collect()
}

/* ─────────────────────────── T-935.7 JSON ⇄ MapLabelsArchive ─────────────────────────── */

/// `locations.json` rows → the archive's `towns` lane, in file order.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn towns_to_archive(locations: &[LocationLabel]) -> Vec<TownLabel> {
    locations
        .iter()
        .map(|l| TownLabel {
            name: l.name.clone(),
            position: [l.x as f32, l.y as f32],
            importance: l.importance as f32,
            kind: l.kind.clone().unwrap_or_default(),
        })
        .collect()
}

/// The archive's `towns` lane → [`LocationLabel`] rows, in archive order.
///
/// `id` comes back empty — see the module table. Callers that need a stable identity use the row
/// index, exactly as `locations_to_label_specs` already does.
#[must_use]
pub fn towns_from_archive(archive: &Archived<MapLabelsArchive>) -> Vec<LocationLabel> {
    archive
        .towns
        .iter()
        .map(|t| LocationLabel {
            id: String::new(),
            name: t.name.to_string(),
            x: f64::from(t.position[0].to_native()),
            y: f64::from(t.position[1].to_native()),
            importance: f64::from(t.importance.to_native()),
            kind: {
                let k = t.kind.as_str();
                if k.is_empty() {
                    None
                } else {
                    Some(k.to_string())
                }
            },
        })
        .collect()
}

/// `height-labels.json` rows → the archive's `height_labels` lane, in file order.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn height_labels_to_archive(labels: &[HeightLabel]) -> Vec<HeightLabelWire> {
    labels
        .iter()
        .map(|l| HeightLabelWire {
            position: [l.x as f32, l.y as f32],
            elevation_m: l.value_m as f32,
        })
        .collect()
}

/// The archive's `height_labels` lane → [`HeightLabel`] rows, in archive order.
///
/// `kind` comes back [`HeightLabelKind::Peak`] and `name` `None` — see the module table. That is
/// the shape the SPA's own height lane produces (`find_peaks` names nothing), so the rows render
/// as the bare elevation either way.
#[must_use]
pub fn height_labels_from_archive(archive: &Archived<MapLabelsArchive>) -> Vec<HeightLabel> {
    archive
        .height_labels
        .iter()
        .map(|h| HeightLabel {
            x: f64::from(h.position[0].to_native()),
            y: f64::from(h.position[1].to_native()),
            #[allow(clippy::cast_possible_truncation)]
            value_m: h.elevation_m.to_native().round() as i32,
            kind: HeightLabelKind::Peak,
            name: None,
        })
        .collect()
}

/* ──────────────────────────── T-935.7 the loader's own read ──────────────────────────── */

/// The alignment a `map_labels.rkyv` buffer must sit on before [`access_checked`] will look at it.
///
/// 16 is what [`to_bytes`](crate::world::binary::to_bytes)' `AlignedVec` writes with, and it is a
/// multiple of every alignment the archived type itself asks for — the `const` below is the proof,
/// checked at compile time rather than trusted.
pub const MAP_LABELS_ALIGN: usize = 16;

const _: () = assert!(
    MAP_LABELS_ALIGN.is_multiple_of(align_of::<Archived<MapLabelsArchive>>()),
    "MAP_LABELS_ALIGN must be a multiple of the archived type's own alignment"
);

/// One `map_labels.rkyv` file, read into the structures the label host renders from.
///
/// `height_labels` is carried because the archive carries it, **not** because the SPA renders it:
/// the SPA's height lane is `find_peaks` over the DEM raster (`world_assets/labels.rs`) and has
/// never read `height-labels.json`. The lane exists here so the emitter's parity pin has a reader,
/// and so a future consumer of the named export has one too.
#[derive(Clone, Debug, PartialEq)]
pub struct MapLabels {
    pub towns: Vec<LocationLabel>,
    pub height_labels: Vec<HeightLabel>,
    /// Candidates in `road_declutter_order`; feed them to
    /// [`build_road_label_draw_set_from_archive`](crate::world::build_road_label_draw_set_from_archive).
    pub road_names: Vec<RoadLabelPlacement>,
}

/// Read raw `map_labels.rkyv` bytes exactly as a loader must: copy onto an alignment
/// [`access_checked`] accepts, **validate**, check the schema version, then materialise the lanes.
///
/// This is the whole of the SPA's binary label path, and it lives here rather than in the wasm-only
/// `world_assets::labels` host so it is reachable from a native test — `world_assets` is
/// `#[cfg(target_arch = "wasm32")]` and this repo has no wasm-bindgen-test harness, so anything
/// that stayed on that side of the boundary would be gated by compilation alone.
///
/// A `Vec<u8>` off the network is 1-aligned, so the copy is not optional; it is one allocation of
/// the file's own size, which for everon is ~3.4 KB against the 13 KB of JSON it replaces.
///
/// # Errors
/// * [`BinaryError::Misaligned`] — the copy could not be placed on the boundary (the allocator
///   refused to say where it put the buffer). Recoverable by falling back to the JSON path.
/// * [`BinaryError::Archive`] — rkyv validation rejected the buffer (truncated, corrupt, not this
///   format).
/// * [`BinaryError::UnsupportedVersion`] — a well-formed archive written by a different schema.
///   `access_checked` cannot catch this: the layout is legal, the *meaning* is not.
pub fn map_labels_from_bytes(raw: &[u8]) -> Result<MapLabels, BinaryError> {
    let (buf, pad) = aligned_copy(raw).ok_or(BinaryError::Misaligned {
        what: "MapLabelsArchive",
        align: MAP_LABELS_ALIGN,
    })?;
    let archive = access_checked::<MapLabelsArchive>(&buf[pad..])?;
    let version = archive.schema_version.to_native();
    if version != ARCHIVE_SCHEMA_VERSION {
        return Err(BinaryError::UnsupportedVersion {
            what: "MapLabelsArchive",
            expected: ARCHIVE_SCHEMA_VERSION,
            actual: version,
        });
    }
    Ok(MapLabels {
        towns: towns_from_archive(archive),
        height_labels: height_labels_from_archive(archive),
        road_names: road_names_from_archive(archive),
    })
}

/// `raw` copied into a buffer whose byte at `pad` sits on [`MAP_LABELS_ALIGN`].
///
/// Capacity is reserved up front so the `extend_from_slice` cannot reallocate and move the
/// alignment out from under the offset that was just measured; the result is re-checked anyway.
fn aligned_copy(raw: &[u8]) -> Option<(Vec<u8>, usize)> {
    let mut buf: Vec<u8> = Vec::with_capacity(raw.len() + MAP_LABELS_ALIGN);
    let pad = buf.as_ptr().align_offset(MAP_LABELS_ALIGN);
    if pad >= MAP_LABELS_ALIGN {
        return None;
    }
    buf.resize(pad, 0);
    buf.extend_from_slice(raw);
    (buf[pad..].as_ptr().align_offset(MAP_LABELS_ALIGN) == 0).then_some((buf, pad))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::binary::to_bytes;

    fn archive_of(towns: Vec<TownLabel>, heights: Vec<HeightLabelWire>) -> rkyv::util::AlignedVec {
        to_bytes(&MapLabelsArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            towns,
            height_labels: heights,
            road_names: Vec::new(),
        })
        .expect("serialise")
    }

    #[test]
    fn parse_sample_row() {
        let json = r#"[
          {"id":"everon-morton","name":"Morton","x":5135.24,"y":4011.78,"importance":0.7,"kind":"village"}
        ]"#;
        let rows = parse_locations_json(json).expect("parse");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Morton");
        let specs = locations_to_label_specs(&rows);
        assert_eq!(specs[0].text, "Morton");
    }

    #[test]
    fn parse_height_labels_reads_the_exporter_shape() {
        let json = r#"[
          {"x":5014.1,"y":8474.51,"value_m":113,"kind":"peak","name":"Center North Hill 01"},
          {"x":1.5,"y":2.5,"value_m":90,"kind":"peak"}
        ]"#;
        let rows = parse_height_labels_json(json).expect("parse");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].value_m, 113);
        assert_eq!(rows[0].name.as_deref(), Some("Center North Hill 01"));
        assert_eq!(rows[0].kind, HeightLabelKind::Peak);
        assert!(rows[1].name.is_none());
    }

    #[test]
    fn parse_height_labels_rejects_a_row_without_coordinates() {
        let err = parse_height_labels_json(r#"[{"x":1.0,"value_m":90}]"#).expect_err("must reject");
        assert!(err.contains("row 0 missing x/y/value_m"), "{err}");
        let err = parse_height_labels_json("{}").expect_err("must reject");
        assert!(err.contains("not an array"), "{err}");
    }

    /// Towns survive the archive as the exact `f32` quantisation of the JSON row — including a
    /// non-ASCII name and a row with no `kind` (the `""` ⇄ `None` edge).
    #[test]
    fn towns_round_trip_through_the_archive() {
        let json = r#"[
          {"id":"everon-entre-deux","name":"Entre Deux","x":5135.24,"y":4011.78,"importance":0.7,"kind":"village"},
          {"id":"everon-saint-philippe","name":"Saint-Philippe é","x":1280.0,"y":6400.5,"importance":0.78}
        ]"#;
        let rows = parse_locations_json(json).expect("parse");
        assert!(
            rows[1].kind.is_none(),
            "fixture must exercise the None kind"
        );
        let bytes = archive_of(towns_to_archive(&rows), Vec::new());
        let back = towns_from_archive(access_checked::<MapLabelsArchive>(&bytes).expect("access"));
        assert_eq!(back.len(), rows.len());
        for (a, b) in back.iter().zip(&rows) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.kind, b.kind);
            assert_eq!(a.x, f64::from(b.x as f32), "{}", b.name);
            assert_eq!(a.y, f64::from(b.y as f32), "{}", b.name);
            assert_eq!(a.importance, f64::from(b.importance as f32));
            assert!(a.id.is_empty(), "the wire type carries no id");
        }
        assert_eq!(
            locations_to_label_specs(&back),
            locations_to_label_specs(&rows),
            "the packed label specs must be identical from either source"
        );
    }

    #[test]
    fn height_labels_round_trip_through_the_archive() {
        let json = r#"[
          {"x":5014.1,"y":8474.51,"value_m":113,"kind":"peak","name":"Center North Hill 01"},
          {"x":3854.08,"y":4344.24,"value_m":163,"kind":"peak"}
        ]"#;
        let rows = parse_height_labels_json(json).expect("parse");
        let bytes = archive_of(Vec::new(), height_labels_to_archive(&rows));
        let back =
            height_labels_from_archive(access_checked::<MapLabelsArchive>(&bytes).expect("access"));
        assert_eq!(back.len(), rows.len());
        for (a, b) in back.iter().zip(&rows) {
            assert_eq!(a.value_m, b.value_m);
            assert_eq!(a.x, f64::from(b.x as f32));
            assert_eq!(a.y, f64::from(b.y as f32));
            assert_eq!(a.kind, HeightLabelKind::Peak);
            assert!(a.name.is_none(), "the wire type carries no name");
        }
    }

    #[test]
    fn empty_lanes_survive_the_round_trip() {
        let bytes = archive_of(Vec::new(), Vec::new());
        let a = access_checked::<MapLabelsArchive>(&bytes).expect("access");
        assert!(towns_from_archive(a).is_empty());
        assert!(height_labels_from_archive(a).is_empty());
    }

    /* ─────────────── T-935.7 the loader's read (`map_labels_from_bytes`) ─────────────── */

    use crate::world::binary::archives::RoadNameLabel;

    /// A three-lane archive, serialised, then handed back the way the network hands a file over:
    /// as a plain `Vec<u8>`, whose data pointer has alignment 1.
    fn three_lane_file() -> Vec<u8> {
        to_bytes(&MapLabelsArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            towns: vec![TownLabel {
                name: "Montignac".into(),
                position: [6400.0, 6400.5],
                importance: 0.875,
                kind: "town".into(),
            }],
            height_labels: vec![HeightLabelWire {
                position: [1024.0, 2048.0],
                elevation_m: 375.0,
            }],
            road_names: vec![RoadNameLabel {
                name: "Main Highway".into(),
                position: [10.5, -20.25],
                angle_deg: 12.5,
                road_class: 1,
            }],
        })
        .expect("serialise")
        .to_vec()
    }

    /// THE LOADER PIN. `fetch_bytes` yields a `Vec<u8>`, not the `AlignedVec` `to_bytes` produced,
    /// and `access_checked` reinterprets in place — so this reads the file the way the SPA gets it
    /// and all three lanes must come back. (The *hostile* offsets are the next test; a `Vec<u8>`
    /// from the allocator is usually already aligned by luck, which is precisely why luck may not
    /// be what the pin rests on.)
    #[test]
    fn a_file_buffer_reads_all_three_lanes() {
        let file = three_lane_file();
        let labels = map_labels_from_bytes(&file).expect("read");
        assert_eq!(labels.towns.len(), 1);
        assert_eq!(labels.towns[0].name, "Montignac");
        assert_eq!(labels.towns[0].y, 6400.5);
        assert_eq!(labels.height_labels.len(), 1);
        assert_eq!(labels.height_labels[0].value_m, 375);
        assert_eq!(labels.road_names.len(), 1);
        assert_eq!(labels.road_names[0].name, "Main Highway");
        assert_eq!(labels.road_names[0].road_class, "highway_paved");
    }

    /// Every byte offset in a 16-byte window: the aligned copy must not depend on where the
    /// allocator happened to put the incoming buffer.
    #[test]
    fn the_read_survives_every_offset_the_allocator_can_hand_it() {
        let file = three_lane_file();
        for skew in 0..MAP_LABELS_ALIGN {
            let mut shifted = vec![0u8; skew];
            shifted.extend_from_slice(&file);
            let labels = map_labels_from_bytes(&shifted[skew..]).expect("read");
            assert_eq!(labels.towns.len(), 1, "skew {skew}");
            assert_eq!(labels.road_names.len(), 1, "skew {skew}");
        }
    }

    /// A truncated download is the common corruption, and it must come back as an error rather
    /// than a wild read — `access_checked` is the only reader, and it validates.
    #[test]
    fn a_truncated_file_is_an_error_not_a_wild_read() {
        let file = three_lane_file();
        for cut in [0, 1, file.len() / 2, file.len() - 1] {
            let err = map_labels_from_bytes(&file[..cut]).expect_err("must reject");
            assert!(
                matches!(err, BinaryError::Archive { .. }),
                "cut {cut}: {err}"
            );
        }
    }

    /// A *legal* archive written by another schema. rkyv validation passes it — the layout is
    /// fine — so the version check is the only thing between it and a mis-rendered map.
    #[test]
    fn a_future_schema_version_is_refused() {
        let file = to_bytes(&MapLabelsArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION + 1,
            towns: Vec::new(),
            height_labels: Vec::new(),
            road_names: Vec::new(),
        })
        .expect("serialise")
        .to_vec();
        let err = map_labels_from_bytes(&file).expect_err("must refuse");
        assert!(
            matches!(
                err,
                BinaryError::UnsupportedVersion {
                    expected: ARCHIVE_SCHEMA_VERSION,
                    ..
                }
            ),
            "{err}"
        );
    }
}
