//! Role: draw order.
//! Position: `core/pipeline` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Lane role.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LaneRole {
    /// Stress.
    Stress,

    /// Calibration.
    Calibration,

    /// Satellite.
    Satellite,

    /// Sea.
    Sea,

    /// Hillshade.
    Hillshade,

    /// land-cover hulls.
    Landcover,

    /// Contours.
    Contours,

    /// World airfield apron.
    WorldAirfieldApron,

    /// Roads casing.
    RoadsCasing,

    /// Roads.
    Roads,

    /// world-building OBB fills (`world-buildings`).
    WorldBuildings,

    /// world-building outline casing (`world-buildings-outline`).
    WorldBuildingsOutline,

    /// World fences.
    WorldFences,

    /// Forest fill.
    ForestFill,

    /// Forest outline.
    ForestOutline,

    /// tree + vegetation glyphs.
    WorldTrees,

    /// prop + rockLarge glyphs.
    WorldProps,

    /// building badges.
    WorldBadges,

    /// World labels.
    WorldLabels,

    /// World road labels.
    WorldRoadLabels,

    /// World town labels.
    WorldTownLabels,

    /// Interior slabs.
    InteriorSlabs,

    /// Interior furniture.
    InteriorFurniture,

    /// Interior furniture outline.
    InteriorFurnitureOutline,

    /// Interior walls.
    InteriorWalls,

    /// Interior walls outline.
    InteriorWallsOutline,

    /// Interior portals.
    InteriorPortals,

    /// Interior portals outline.
    InteriorPortalsOutline,

    /// Interior glazing.
    InteriorGlazing,

    /// Interior glazing outline.
    InteriorGlazingOutline,

    /// Interior stairs.
    InteriorStairs,

    /// Scene vegetation.
    SceneVegetation,

    /// Scene vegetation outline.
    SceneVegetationOutline,

    /// Viewshed.
    Viewshed,

    /// Interior probe.
    InteriorProbe,

    /// Order: above `Grid` (zones are mission data, not world chrome) and below `SquadLinks`, so a zone ring can enclose the units it contains without ever occluding a slot marker.
    MissionZones,

    /// Mission markers.
    MissionMarkers,

    /// Mission comments.
    MissionComments,

    /// Order: above `MissionComments` (an edge is mission data, and it must composite over the annotation glyphs it may pass under) and below `SquadLinks` — the squad hairlines are structural ORBAT truth and win the overprint, exactly as every mission lane loses to `Slots`, so a connection can never occlude a unit ring.
    MissionConnections,

    /// Squad links.
    SquadLinks,

    /// Mission vehicles.
    MissionVehicles,

    /// mission slot rings.
    Slots,

    /// Slot place preview.
    SlotPlacePreview,

    /// Slot drag.
    SlotDrag,

    /// Clusters.
    Clusters,

    /// Grid.
    Grid,

    /// Selection marquee fill (topmost with its outline).
    Marquee,

    /// Marquee outline.
    MarqueeOutline,
}

/// Lane order.
pub fn lane_order(role: LaneRole) -> u8 {
    match role {
        LaneRole::Stress | LaneRole::Calibration => 0,
        LaneRole::Satellite => 1,
        LaneRole::Sea => 2,
        LaneRole::Hillshade => 3,
        LaneRole::Landcover => 4,
        LaneRole::Contours => 5,
        LaneRole::WorldAirfieldApron => 6,
        LaneRole::RoadsCasing => 7,
        LaneRole::Roads => 8,
        LaneRole::WorldBuildings => 9,
        LaneRole::WorldBuildingsOutline => 10,
        LaneRole::WorldFences => 11,
        LaneRole::ForestFill => 12,
        LaneRole::ForestOutline => 13,
        LaneRole::WorldTrees => 15,
        LaneRole::WorldProps => 16,
        LaneRole::WorldBadges => 17,
        LaneRole::WorldLabels => 18,
        LaneRole::WorldRoadLabels => 19,
        LaneRole::WorldTownLabels => 20,

        LaneRole::InteriorSlabs => 21,
        LaneRole::InteriorFurniture => 22,
        LaneRole::InteriorFurnitureOutline => 23,
        LaneRole::InteriorWalls => 24,
        LaneRole::InteriorWallsOutline => 25,
        LaneRole::InteriorPortals => 26,
        LaneRole::InteriorPortalsOutline => 27,
        LaneRole::InteriorGlazing => 28,
        LaneRole::InteriorGlazingOutline => 29,
        LaneRole::InteriorStairs => 30,
        LaneRole::SceneVegetation => 31,
        LaneRole::SceneVegetationOutline => 32,

        LaneRole::Viewshed => 33,
        LaneRole::InteriorProbe => 34,
        LaneRole::Grid => 35,
        LaneRole::MissionZones => 36,
        LaneRole::MissionMarkers => 37,
        LaneRole::MissionComments => 38,
        LaneRole::MissionConnections => 39,
        LaneRole::SquadLinks => 40,
        LaneRole::MissionVehicles => 41,
        LaneRole::Slots => 42,
        LaneRole::SlotPlacePreview => 43,
        LaneRole::SlotDrag => 44,
        LaneRole::Clusters => 45,
        LaneRole::Marquee => 46,
        LaneRole::MarqueeOutline => 47,
    }
}

/// Canonical all lanes value.
pub const ALL_LANES: [LaneRole; 48] = [
    LaneRole::Stress,
    LaneRole::Calibration,
    LaneRole::Satellite,
    LaneRole::Sea,
    LaneRole::Hillshade,
    LaneRole::Landcover,
    LaneRole::Contours,
    LaneRole::WorldAirfieldApron,
    LaneRole::RoadsCasing,
    LaneRole::Roads,
    LaneRole::WorldBuildings,
    LaneRole::WorldBuildingsOutline,
    LaneRole::WorldFences,
    LaneRole::ForestFill,
    LaneRole::ForestOutline,
    LaneRole::WorldTrees,
    LaneRole::WorldProps,
    LaneRole::WorldBadges,
    LaneRole::WorldLabels,
    LaneRole::WorldRoadLabels,
    LaneRole::WorldTownLabels,
    LaneRole::InteriorSlabs,
    LaneRole::InteriorFurniture,
    LaneRole::InteriorFurnitureOutline,
    LaneRole::InteriorWalls,
    LaneRole::InteriorWallsOutline,
    LaneRole::InteriorPortals,
    LaneRole::InteriorPortalsOutline,
    LaneRole::InteriorGlazing,
    LaneRole::InteriorGlazingOutline,
    LaneRole::InteriorStairs,
    LaneRole::SceneVegetation,
    LaneRole::SceneVegetationOutline,
    LaneRole::Viewshed,
    LaneRole::InteriorProbe,
    LaneRole::Grid,
    LaneRole::MissionZones,
    LaneRole::MissionMarkers,
    LaneRole::MissionComments,
    LaneRole::MissionConnections,
    LaneRole::SquadLinks,
    LaneRole::MissionVehicles,
    LaneRole::Slots,
    LaneRole::SlotPlacePreview,
    LaneRole::SlotDrag,
    LaneRole::Clusters,
    LaneRole::Marquee,
    LaneRole::MarqueeOutline,
];

/// Public role ids for the **vector-lane upload API** (`upload_polygon_mesh`, `upload_strip_tris`, `upload_hairline_segments`, `clear_vector_lane`).
pub use super::roles::role_id;

/// Lane role from u32.
pub fn lane_role_from_u32(role: u32) -> Option<LaneRole> {
    Some(match role {
        role_id::SEA => LaneRole::Sea,
        role_id::LANDCOVER => LaneRole::Landcover,
        role_id::CONTOURS => LaneRole::Contours,
        role_id::AIRFIELD_APRON => LaneRole::WorldAirfieldApron,
        role_id::ROADS_CASING => LaneRole::RoadsCasing,
        role_id::ROADS => LaneRole::Roads,
        role_id::FOREST_FILL => LaneRole::ForestFill,
        role_id::FOREST_OUTLINE => LaneRole::ForestOutline,
        role_id::MARQUEE => LaneRole::Marquee,
        role_id::SQUAD_LINKS => LaneRole::SquadLinks,
        role_id::MISSION_ZONES => LaneRole::MissionZones,
        role_id::INTERIOR_SLABS => LaneRole::InteriorSlabs,
        role_id::INTERIOR_FURNITURE => LaneRole::InteriorFurniture,
        role_id::INTERIOR_FURNITURE_OUTLINE => LaneRole::InteriorFurnitureOutline,
        role_id::INTERIOR_WALLS => LaneRole::InteriorWalls,
        role_id::INTERIOR_WALLS_OUTLINE => LaneRole::InteriorWallsOutline,
        role_id::INTERIOR_PORTALS => LaneRole::InteriorPortals,
        role_id::INTERIOR_PORTALS_OUTLINE => LaneRole::InteriorPortalsOutline,
        role_id::INTERIOR_GLAZING => LaneRole::InteriorGlazing,
        role_id::INTERIOR_GLAZING_OUTLINE => LaneRole::InteriorGlazingOutline,
        role_id::INTERIOR_STAIRS => LaneRole::InteriorStairs,
        role_id::SCENE_VEGETATION => LaneRole::SceneVegetation,
        role_id::SCENE_VEGETATION_OUTLINE => LaneRole::SceneVegetationOutline,
        role_id::INTERIOR_PROBE => LaneRole::InteriorProbe,
        _ => return None,
    })
}

/// Inverse of [`lane_role_from_u32`]. `None` for the engine-internal lanes that have no upload id (spike batches, textured lanes, residency-composed world lanes, the mission lanes fed by their own typed APIs). Exists so the round-trip can be proved exhaustive in both directions rather than spot-checked — see `wire_round_trip_is_exhaustive_both_ways`.
pub fn lane_role_to_u32(role: LaneRole) -> Option<u32> {
    Some(match role {
        LaneRole::Sea => role_id::SEA,
        LaneRole::Landcover => role_id::LANDCOVER,
        LaneRole::Contours => role_id::CONTOURS,
        LaneRole::WorldAirfieldApron => role_id::AIRFIELD_APRON,
        LaneRole::RoadsCasing => role_id::ROADS_CASING,
        LaneRole::Roads => role_id::ROADS,
        LaneRole::ForestFill => role_id::FOREST_FILL,
        LaneRole::ForestOutline => role_id::FOREST_OUTLINE,
        LaneRole::Marquee => role_id::MARQUEE,
        LaneRole::SquadLinks => role_id::SQUAD_LINKS,
        LaneRole::MissionZones => role_id::MISSION_ZONES,
        LaneRole::InteriorSlabs => role_id::INTERIOR_SLABS,
        LaneRole::InteriorFurniture => role_id::INTERIOR_FURNITURE,
        LaneRole::InteriorFurnitureOutline => role_id::INTERIOR_FURNITURE_OUTLINE,
        LaneRole::InteriorWalls => role_id::INTERIOR_WALLS,
        LaneRole::InteriorWallsOutline => role_id::INTERIOR_WALLS_OUTLINE,
        LaneRole::InteriorPortals => role_id::INTERIOR_PORTALS,
        LaneRole::InteriorPortalsOutline => role_id::INTERIOR_PORTALS_OUTLINE,
        LaneRole::InteriorGlazing => role_id::INTERIOR_GLAZING,
        LaneRole::InteriorGlazingOutline => role_id::INTERIOR_GLAZING_OUTLINE,
        LaneRole::InteriorStairs => role_id::INTERIOR_STAIRS,
        LaneRole::SceneVegetation => role_id::SCENE_VEGETATION,
        LaneRole::SceneVegetationOutline => role_id::SCENE_VEGETATION_OUTLINE,
        LaneRole::InteriorProbe => role_id::INTERIOR_PROBE,

        LaneRole::Stress
        | LaneRole::Calibration
        | LaneRole::Satellite
        | LaneRole::Hillshade
        | LaneRole::WorldBuildings
        | LaneRole::WorldBuildingsOutline
        | LaneRole::WorldFences
        | LaneRole::WorldTrees
        | LaneRole::WorldProps
        | LaneRole::WorldBadges
        | LaneRole::WorldLabels
        | LaneRole::WorldRoadLabels
        | LaneRole::WorldTownLabels
        | LaneRole::Viewshed
        | LaneRole::Grid
        | LaneRole::MissionMarkers
        | LaneRole::MissionComments
        | LaneRole::MissionConnections
        | LaneRole::MissionVehicles
        | LaneRole::Slots
        | LaneRole::SlotPlacePreview
        | LaneRole::SlotDrag
        | LaneRole::Clusters
        | LaneRole::MarqueeOutline => return None,
    })
}

/// Public role ids for the **texture-lane API** (`tex_layer_begin` / `tex_layer_commit` / `tex_layer_clear` / `set_lane_opacity`). A **disjoint** namespace from [`role_id`]: these index a fixed `[Option<PendingTex>; 2]` bucket in the engine, and id `0` means `Satellite` here but `Sea` over there.
pub use super::roles::tex_role_id;

/// Map a texture-lane role u32 → [`LaneRole`]. `None` for anything that is not `0` or `1`.
pub fn tex_lane_role_from_u32(role: u32) -> Option<LaneRole> {
    Some(match role {
        tex_role_id::BASEMAP => LaneRole::Satellite,
        tex_role_id::HILLSHADE => LaneRole::Hillshade,
        _ => return None,
    })
}

#[cfg(test)]
#[path = "tests/draw_order.rs"]
mod lane_order_pins;
