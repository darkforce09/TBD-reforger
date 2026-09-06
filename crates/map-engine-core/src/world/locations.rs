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
use crate::world::binary::archives::{HeightLabel as HeightLabelWire, MapLabelsArchive, TownLabel};
use crate::world::importance_declutter::LocationLabel;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::binary::archives::ARCHIVE_SCHEMA_VERSION;
    use crate::world::binary::{access_checked, to_bytes};

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
}
