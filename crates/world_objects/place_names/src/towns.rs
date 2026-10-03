//! Town and height labels from their JSON files and from the labels archive.
//!
//! **Role:** parses `locations.json` and `height-labels.json` ([`parse_locations_json`],
//! [`parse_height_labels_json`]), turns location rows into label specs
//! ([`locations_to_label_specs`]), converts the town and height lanes of the labels archive both
//! ways, and reads a whole `map_labels.rkyv` file ([`map_labels_from_bytes`]).
//! **Position:** reads `label_layout`'s location rows and `world_file_formats`' labels archive;
//! the road lane comes from [`crate::route_labels`]. The map engine's label loader and the
//! developer tools' label archive builder and checks read it.
//! **Signals & state:** none; pure functions over their arguments.
//! **Invariants:** a lane converts in file order and reads back in archive order; the archive is
//! validated and its schema version checked before any lane is read, from any buffer offset.

#![forbid(unsafe_code)]

use label_layout::declutter::LabelSpec;
use label_layout::importance::LocationLabel;
use label_layout::label_ids::{LabelId, LocationId};
use rkyv::Archived;
use world_file_formats::archives::codec::BinaryError;
use world_file_formats::archives::codec::access_checked;
use world_file_formats::archives::labels::HeightLabel as HeightLabelWire;
use world_file_formats::archives::labels::MapLabelsArchive;
use world_file_formats::archives::labels::TownLabel;
use world_file_formats::archives::version::ARCHIVE_SCHEMA_VERSION;

use crate::error::{Error, Result};
use crate::peaks::HeightLabel;
use crate::peaks::HeightLabelKind;
use crate::route_labels::road_names_from_archive;
use crate::route_placement::RoadLabelPlacement;

/// Parse a `locations.json` array payload.
///
/// # Errors
/// [`Error::LocationsJson`] when the payload is not an array of location rows.
pub fn parse_locations_json(json: &str) -> Result<Vec<LocationLabel>> {
    serde_json::from_str(json).map_err(Error::LocationsJson)
}

/// Parse a `height-labels.json` array payload into [`HeightLabel`] rows.
///
/// # Errors
/// [`Error::HeightLabelsJson`] when the payload is not JSON, [`Error::HeightLabelsNotAnArray`]
/// when its root is not an array, and [`Error::HeightLabelRowIncomplete`] for the first row
/// without a numeric `x`, `y` or `value_m`.
pub fn parse_height_labels_json(json: &str) -> Result<Vec<HeightLabel>> {
    let raw: serde_json::Value = serde_json::from_str(json).map_err(Error::HeightLabelsJson)?;
    let rows = raw.as_array().ok_or(Error::HeightLabelsNotAnArray)?;
    let mut out = Vec::with_capacity(rows.len());
    for (i, r) in rows.iter().enumerate() {
        let num = |k: &str| r.get(k).and_then(serde_json::Value::as_f64);
        let (Some(x), Some(y), Some(value_m)) = (num("x"), num("y"), num("value_m")) else {
            return Err(Error::HeightLabelRowIncomplete { row: i });
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
            id: LabelId::new(i as u32),
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
            id: LocationId::new(String::new()),
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

/// 16 is what [`to_bytes`](world_file_formats::archives::codec::to_bytes)' `AlignedVec` writes with, and it is a multiple of every alignment the archived type itself asks for — the `const` below is the proof, checked at compile time rather than trusted.
pub const MAP_LABELS_ALIGN: usize = 16;

const _: () = assert!(
    MAP_LABELS_ALIGN.is_multiple_of(align_of::<Archived<MapLabelsArchive>>()),
    "MAP_LABELS_ALIGN must be a multiple of the archived type's own alignment"
);

/// One `map_labels.rkyv` file, read into the structures the label host renders from.
#[derive(Clone, Debug, PartialEq)]
pub struct MapLabels {
    /// Town and place name labels of the archive's town lane, in archive order.
    pub towns: Vec<LocationLabel>,

    /// Spot heights of the archive's height lane, each read back as an unnamed peak.
    pub height_labels: Vec<HeightLabel>,

    /// Candidates in `road_declutter_order`; feed them to [`build_road_label_draw_set_from_archive`](crate::route_labels::build_road_label_draw_set_from_archive).
    pub road_names: Vec<RoadLabelPlacement>,
}

/// Read raw `map_labels.rkyv` bytes exactly as a loader must: copy onto an alignment [`access_checked`] accepts, **validate**, check the schema version, then materialise the lanes.
///
/// # Errors
/// [`Error::Binary`] holding [`BinaryError::Misaligned`] when no aligned copy can be made,
/// the archive's validation error when the bytes are malformed or truncated, and
/// [`BinaryError::UnsupportedVersion`] for another schema version.
pub fn map_labels_from_bytes(raw: &[u8]) -> Result<MapLabels> {
    let (buf, pad) = aligned_copy(raw).ok_or(BinaryError::Misaligned {
        what: "MapLabelsArchive",
        align: MAP_LABELS_ALIGN,
    })?;
    let archive = access_checked::<MapLabelsArchive>(&buf[pad..])?;
    let version = archive.schema_version.to_native();
    if version != ARCHIVE_SCHEMA_VERSION {
        return Err(Error::Binary(BinaryError::UnsupportedVersion {
            what: "MapLabelsArchive",
            expected: ARCHIVE_SCHEMA_VERSION,
            actual: version,
        }));
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
