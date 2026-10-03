//! The `road-names.json` model and the labels archive's road name lane.
//!
//! **Role:** parses a terrain's curated road names ([`parse_road_names_json`]), bakes their placed
//! anchors into the archive's `road_names` lane ([`road_names_to_archive`]), reads the lane back
//! as candidates ([`road_names_from_archive`]) and draws them at a zoom
//! ([`build_road_label_draw_set_from_archive`]).
//! **Position:** reads [`crate::route_placement`], `road_network`'s segments and class codec and
//! `world_file_formats`' labels archive; [`crate::towns`], the map engine's label loader and the
//! developer tools' label archive builder and checks read it.
//! **Signals & state:** none; plain data and pure functions.
//! **Invariants:** the lane is baked at the highest zoom floor any entry names and stored in
//! declutter order, so drawing it only filters and declutters; a name's floor travels as the
//! one-byte code of a road class with that floor, and a floor no class has is refused.

use crate::route_placement::ROAD_NAME_MIN_ZOOM_SECONDARY;
use crate::route_placement::RoadLabelPlacement;
use crate::route_placement::declutter_road_labels_in_order;
use crate::route_placement::place_road_labels;
use crate::route_placement::road_class_priority;
use crate::route_placement::road_class_visibility_floor;
use crate::route_placement::road_declutter_order;
use crate::route_placement::road_name_visible_for_class;
use road_network::network::RoadSegment;
use road_network::road_class::ROAD_CLASSES;
use road_network::road_class::road_class_code;
use road_network::road_class::road_class_name;
use world_file_formats::ids::{RoadSegmentId, TerrainId};

use crate::error::{Error, Result};
use crate::road_name_ids::RoadNameId;

/// One curated named route from `road-names.json`.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RoadNameEntry {
    /// The entry's identifier.
    pub id: RoadNameId,

    /// The road name drawn along the segments (`name`), such as `Main Highway`.
    pub name: String,

    /// The road segments the name is placed on.
    #[serde(rename = "segmentIds")]
    pub segment_ids: Vec<RoadSegmentId>,

    /// Optional visibility floor (curated major routes pin `0` per G3 @ z=0).
    #[serde(rename = "minDeckZoom", default)]
    pub min_deck_zoom: Option<f64>,
}

/// Payload root for `road-names.json`.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RoadNamesFile {
    /// Version of the `road-names.json` layout (`schemaVersion`), such as `1.0.0`.
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,

    /// The terrain the names belong to.
    #[serde(rename = "terrainId")]
    pub terrain_id: TerrainId,

    /// The curated named routes (`roads`), in file order.
    pub roads: Vec<RoadNameEntry>,
}

/// Parse `road-names.json`.
///
/// # Errors
/// [`Error::RoadNamesJson`] when the payload does not match [`RoadNamesFile`].
pub fn parse_road_names_json(json: &str) -> Result<RoadNamesFile> {
    serde_json::from_str(json).map_err(Error::RoadNamesJson)
}

/// G4: every name has length ≥ 2.
#[must_use]
pub fn road_name_schema_holds(drawn: &[RoadLabelPlacement]) -> bool {
    drawn.iter().all(|l| l.name.trim().len() >= 2)
}

/// `road-names.json` + centrelined segments → the archive's `road_names` lane.
///
/// # Errors
/// [`Error::RoadClassNeverDrawn`] for a name without its own floor on a class that never draws,
/// and [`Error::UnrepresentableVisibilityFloor`] for a floor no road class has.
pub fn road_names_to_archive(
    names: &RoadNamesFile,
    segments: &[RoadSegment],
) -> Result<Vec<world_file_formats::archives::labels::RoadNameLabel>> {
    let bake_zoom = names
        .roads
        .iter()
        .filter_map(|e| e.min_deck_zoom)
        .fold(ROAD_NAME_MIN_ZOOM_SECONDARY, f64::max);
    let mut baked: Vec<(RoadLabelPlacement, u8)> = Vec::new();
    for entry in &names.roads {
        let one = RoadNamesFile {
            schema_version: names.schema_version.clone(),
            terrain_id: names.terrain_id.clone(),
            roads: vec![entry.clone()],
        };
        for cand in place_road_labels(&one, segments, bake_zoom) {
            let floor = match entry.min_deck_zoom {
                Some(m) => m,
                None => road_class_visibility_floor(&cand.road_class).ok_or_else(|| {
                    Error::RoadClassNeverDrawn {
                        name: cand.name.clone(),
                        road_class: cand.road_class.clone(),
                    }
                })?,
            };
            let gate_class = ROAD_CLASSES
                .iter()
                .find(|c| road_class_visibility_floor(c) == Some(floor))
                .ok_or_else(|| Error::UnrepresentableVisibilityFloor {
                    name: cand.name.clone(),
                    floor,
                })?;
            baked.push((cand, road_class_code(gate_class)));
        }
    }
    baked.sort_by(|a, b| road_declutter_order(&a.0, &b.0));
    #[allow(clippy::cast_possible_truncation)]
    Ok(baked
        .into_iter()
        .map(
            |(p, code)| world_file_formats::archives::labels::RoadNameLabel {
                name: p.name,
                position: [p.x as f32, p.y as f32],
                angle_deg: p.angle_deg as f32,
                road_class: code,
            },
        )
        .collect())
}

/// The archive's `road_names` lane → candidate [`RoadLabelPlacement`]s, still in the declutter
/// order (priority descending, then name, then segment id). Feed them to
/// [`build_road_label_draw_set_from_archive`].
#[must_use]
pub fn road_names_from_archive(
    archive: &rkyv::Archived<world_file_formats::archives::labels::MapLabelsArchive>,
) -> Vec<RoadLabelPlacement> {
    archive
        .road_names
        .iter()
        .map(|r| {
            let road_class = road_class_name(r.road_class);
            RoadLabelPlacement {
                name: r.name.to_string(),
                x: f64::from(r.position[0].to_native()),
                y: f64::from(r.position[1].to_native()),
                angle_deg: f64::from(r.angle_deg.to_native()),
                priority: road_class_priority(road_class),
                segment_id: RoadSegmentId::new(String::new()),
                road_class: road_class.to_string(),
                arc_frac: 0.0,
            }
        })
        .collect()
}

/// [`build_road_label_draw_set`](crate::route_placement::build_road_label_draw_set) for
/// archive-derived candidates: gate by the wire class's visibility floor, then declutter
/// **without re-sorting** (the lane is already ordered).
#[must_use]
pub fn build_road_label_draw_set_from_archive(
    candidates: &[RoadLabelPlacement],
    deck_zoom: f64,
) -> Vec<RoadLabelPlacement> {
    let visible: Vec<RoadLabelPlacement> = candidates
        .iter()
        .filter(|c| road_name_visible_for_class(&c.road_class, deck_zoom))
        .cloned()
        .collect();
    declutter_road_labels_in_order(&visible, deck_zoom)
}
