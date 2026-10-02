//! Role: route labels.
//! Position: `world/environment/locations` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::environment::locations::route_placement::ROAD_CLASSES;
use crate::world::environment::locations::route_placement::ROAD_NAME_MIN_ZOOM_SECONDARY;
use crate::world::environment::locations::route_placement::RoadLabelPlacement;
use crate::world::environment::locations::route_placement::declutter_road_labels_in_order;
use crate::world::environment::locations::route_placement::place_road_labels;
use crate::world::environment::locations::route_placement::road_class_code;
use crate::world::environment::locations::route_placement::road_class_name;
use crate::world::environment::locations::route_placement::road_class_priority;
use crate::world::environment::locations::route_placement::road_class_visibility_floor;
use crate::world::environment::locations::route_placement::road_declutter_order;
use crate::world::environment::locations::route_placement::road_name_visible_for_class;
use crate::world::terrain::roads::network::RoadSegment;

/// One curated named route from `road-names.json`.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RoadNameEntry {
    /// Id.
    pub id: String,

    /// Name.
    pub name: String,

    /// Segment ids.
    #[serde(rename = "segmentIds")]
    pub segment_ids: Vec<String>,

    /// Optional visibility floor (curated major routes pin `0` per G3 @ z=0).
    #[serde(rename = "minDeckZoom", default)]
    pub min_deck_zoom: Option<f64>,
}

/// Payload root for `road-names.json`.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RoadNamesFile {
    /// Schema version.
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,

    /// Terrain id.
    #[serde(rename = "terrainId")]
    pub terrain_id: String,

    /// Roads.
    pub roads: Vec<RoadNameEntry>,
}

/// Parse `road-names.json`.
pub fn parse_road_names_json(json: &str) -> Result<RoadNamesFile, String> {
    serde_json::from_str(json).map_err(|e| format!("road-names json: {e}"))
}

/// G4: every name has length ≥ 2.
#[must_use]
pub fn road_name_schema_holds(drawn: &[RoadLabelPlacement]) -> bool {
    drawn.iter().all(|l| l.name.trim().len() >= 2)
}

/// `road-names.json` + centrelined segments → the archive's `road_names` lane.
pub fn road_names_to_archive(
    names: &RoadNamesFile,
    segments: &[RoadSegment],
) -> Result<Vec<crate::io::archives::labels::RoadNameLabel>, String> {
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
                    format!(
                        "road-names archive: \"{}\" is class {} which is never drawn",
                        cand.name, cand.road_class
                    )
                })?,
            };
            let gate_class = ROAD_CLASSES
                .iter()
                .find(|c| road_class_visibility_floor(c) == Some(floor))
                .ok_or_else(|| {
                    format!(
                        "road-names archive: \"{}\" needs visibility floor {floor}, which no road \
                         class expresses — RoadNameLabel cannot carry it",
                        cand.name
                    )
                })?;
            baked.push((cand, road_class_code(gate_class)));
        }
    }
    baked.sort_by(|a, b| road_declutter_order(&a.0, &b.0));
    #[allow(clippy::cast_possible_truncation)]
    Ok(baked
        .into_iter()
        .map(|(p, code)| crate::io::archives::labels::RoadNameLabel {
            name: p.name,
            position: [p.x as f32, p.y as f32],
            angle_deg: p.angle_deg as f32,
            road_class: code,
        })
        .collect())
}

/// The archive's `road_names` lane → candidate [`RoadLabelPlacement`]s, still in [`road_declutter_order`]. Feed them to [`build_road_label_draw_set_from_archive`].
#[must_use]
pub fn road_names_from_archive(
    archive: &rkyv::Archived<crate::io::archives::labels::MapLabelsArchive>,
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
                segment_id: String::new(),
                road_class: road_class.to_string(),
                arc_frac: 0.0,
            }
        })
        .collect()
}

/// [`build_road_label_draw_set`] for archive-derived candidates: gate by the wire class's visibility floor, then declutter **without re-sorting** (the lane is already ordered).
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
