//! Role: what an armed palette pick-up commits into the document when the canvas is released.
//! Position: `doc/operations/entity` in the map engine's headless mission data domain.
//! Signals & state: the armed value is passed in and consumed; the host keeps it until then.
//! Invariants: a release commits at most one entity, and a commit that any step refuses leaves
//! the document exactly as it was. The id is minted before the commit is attempted, so a refused
//! release costs an id rather than risking a second placement reusing one.

use std::cell::Cell;

use super::MissionDocCore;
use super::PlacePayload;
use super::marker_rows_of;
use super::mint_id;
use super::mint_marker_id;
use super::place_character_under_side;
use super::place_object_in_core;
use super::place_saved_composition;
use super::place_vehicle_in_core;
use super::side_faction_id;
use crate::data::store::operations::cargo::seed_cargo_for_asset;

/// What a palette leaf picked up, held until a canvas release commits it.
///
/// The discriminant rides the armed value rather than a separate "current tab" reading, because
/// the tab can change between the pick-up and the release and a release must commit the thing the
/// operator actually picked up.
#[derive(Clone, Debug, PartialEq)]
pub enum ArmedPlacement {
    /// A character, placed under the active side's order of battle and into the active layer.
    Character(PlacePayload),

    /// A vehicle, placed for the active side.
    Vehicle(PlacePayload),

    /// A world object.
    Object(PlacePayload),

    /// A saved composition, stamped by id.
    Composition(String),

    /// A briefing marker, by icon name.
    Marker(String),

    /// A zone or trigger draw. A release routes to the draw machine rather than here, so this arm
    /// commits nothing; it exists because one armed cell holds every kind of arm.
    ZoneDraw,
}

/// The details captured when an armed palette value is released onto the map.
pub struct ArmedPlacementRequest<'a> {
    pub armed: ArmedPlacement,
    pub side: &'a str,
    pub x: f64,
    pub y: f64,
    pub crew_toggle: bool,
    pub alt_empty: bool,
}

/// What a release put into the document, and what the host still owes it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlacementCommit {
    /// The ids the placement leaves selected, when it selects anything. A marker, a vehicle and an
    /// object leave the selection alone: none of them is a slot, and the selection is the slot
    /// selection.
    pub selection: Option<Vec<String>>,

    /// Whether a vehicle reached the document, so the host can rebind the vehicle symbology lane
    /// it owns.
    pub placed_vehicle: bool,

    /// `(composition id, title)` when a saved composition was stamped, for the host's
    /// recently-placed list.
    pub stamped_composition: Option<(String, String)>,
}

/// Does a released vehicle place bring its crew?
///
/// The session's crew toggle says what the operator wants by default; holding the alternate
/// modifier over a single release says "this one empty", which is the faster gesture than reaching
/// for the toggle and back. The modifier can only ever REMOVE the crew, so an empty place is never
/// a surprise.
#[must_use]
pub fn vehicle_places_its_crew(crew_toggle: bool, alt_empty: bool) -> bool {
    crew_toggle && !alt_empty
}

/// Commit an armed value at the request's world position for its side.
///
/// `ensure_layer` resolves the layer a placed character or composition is filed into — it is the
/// host's because the active layer is a host reading, and it may have to mint the default layer.
/// `crew_toggle` and `alt_empty` are the two halves of [`vehicle_places_its_crew`].
///
/// `None` when nothing was committed: the arm was a draw, the document refused the placement, or
/// the composition carried no entities.
pub fn commit_armed_placement(
    core: &MissionDocCore,
    request: ArmedPlacementRequest<'_>,
    next_id: &Cell<u32>,
    ensure_layer: impl FnOnce(&MissionDocCore) -> String,
) -> Option<PlacementCommit> {
    let ArmedPlacementRequest {
        armed,
        side,
        x,
        y,
        crew_toggle,
        alt_empty,
    } = request;
    let id = mint_id(core, next_id);
    match armed {
        ArmedPlacement::ZoneDraw => None,

        ArmedPlacement::Vehicle(payload) => {
            let with_crew = vehicle_places_its_crew(crew_toggle, alt_empty);
            if !place_vehicle_in_core(core, side, &id, &payload.asset_id, x, y, with_crew) {
                return None;
            }
            Some(PlacementCommit {
                placed_vehicle: true,
                ..PlacementCommit::default()
            })
        }

        ArmedPlacement::Object(payload) => {
            if !place_object_in_core(core, side, &id, &payload, x, y) {
                return None;
            }
            Some(PlacementCommit::default())
        }

        ArmedPlacement::Marker(icon) => {
            let faction_id = side_faction_id(side);
            let marker_id = mint_marker_id(&marker_rows_of(core));
            core.set_faction_briefing_marker(&faction_id, &marker_id, x, y, &icon, "");
            Some(PlacementCommit::default())
        }

        ArmedPlacement::Composition(comp_id) => {
            let (slot_ids, title) =
                place_saved_composition(core, &comp_id, side, x, y, next_id, ensure_layer)?;
            Some(PlacementCommit {
                selection: Some(slot_ids),
                stamped_composition: Some((comp_id, title)),
                ..PlacementCommit::default()
            })
        }

        ArmedPlacement::Character(payload) => {
            let layer_id = ensure_layer(core);
            let asset_id = payload.asset_id.clone();
            place_character_under_side(
                core,
                side,
                &id,
                &layer_id,
                &payload.role,
                None,
                Some(payload.asset_id),
                x,
                y,
                0.0,
                0.0,
            )
            .ok()?;

            // A character placed from the palette starts with the cargo its asset is authored to
            // carry, so the operator opens the Arsenal on a kitted slot rather than an empty one.
            seed_cargo_for_asset(core, &id, &asset_id, None);
            Some(PlacementCommit {
                selection: Some(vec![id]),
                ..PlacementCommit::default()
            })
        }
    }
}

#[cfg(test)]
#[path = "tests/armed_placement.rs"]
mod tests;
