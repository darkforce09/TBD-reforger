//! Role: towns.
//! Position: `world/environment/locations` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

#![forbid(unsafe_code)]

use crate::io::archives::codec::BinaryError;
use crate::io::archives::codec::access_checked;
use crate::io::archives::labels::HeightLabel as HeightLabelWire;
use crate::io::archives::labels::MapLabelsArchive;
use crate::io::archives::labels::TownLabel;
use crate::io::archives::version::ARCHIVE_SCHEMA_VERSION;
use crate::overlay::symbology::labels::declutter::LabelSpec;
use crate::overlay::symbology::labels::importance::LocationLabel;
use crate::world::environment::locations::peaks::HeightLabel;
use crate::world::environment::locations::peaks::HeightLabelKind;
use crate::world::environment::locations::route_labels::road_names_from_archive;
use crate::world::environment::locations::route_placement::RoadLabelPlacement;

use rkyv::Archived;

/// Parse a `locations.json` array payload.
pub fn parse_locations_json(json: &str) -> Result<Vec<LocationLabel>, String> {
    serde_json::from_str(json).map_err(|e| format!("locations json: {e}"))
}

/// Parse a `height-labels.json` array payload into [`HeightLabel`] rows.
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

/// 16 is what [`to_bytes`](crate::world::binary::to_bytes)' `AlignedVec` writes with, and it is a multiple of every alignment the archived type itself asks for — the `const` below is the proof, checked at compile time rather than trusted.
pub const MAP_LABELS_ALIGN: usize = 16;

const _: () = assert!(
    MAP_LABELS_ALIGN.is_multiple_of(align_of::<Archived<MapLabelsArchive>>()),
    "MAP_LABELS_ALIGN must be a multiple of the archived type's own alignment"
);

/// One `map_labels.rkyv` file, read into the structures the label host renders from.
#[derive(Clone, Debug, PartialEq)]
pub struct MapLabels {
    /// Towns.
    pub towns: Vec<LocationLabel>,

    /// Height labels.
    pub height_labels: Vec<HeightLabel>,

    /// Candidates in `road_declutter_order`; feed them to [`build_road_label_draw_set_from_archive`](crate::world::build_road_label_draw_set_from_archive).
    pub road_names: Vec<RoadLabelPlacement>,
}

/// Read raw `map_labels.rkyv` bytes exactly as a loader must: copy onto an alignment [`access_checked`] accepts, **validate**, check the schema version, then materialise the lanes.
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
#[path = "tests/towns_tests.rs"]
mod tests;
