//! The 48 named lane identities, their paint order, and the two public id namespaces.
//!
//! **Role:** names every lane the map draws ([`LaneRole`]), ranks them into paint order
//! ([`lane_order`], [`ALL_LANES`]), turns a role into the renderer's opaque
//! `render_primitives::frame::ids::LaneId` ([`lane_id`]), and maps the browser API's wire ids
//! ([`role_id`], [`tex_role_id`]) to roles and back.
//! **Position:** `map_draw_lanes`, tier 1 over `render_primitives`; the map engine's frame
//! builder, residency, editing lanes and diagnostics read it, and the overlay crates name the
//! lanes their marks land on.
//! **Signals & state:** none; names, a rank table and constant wire ids.
//! **Invariants:** paint order is the explicit rank in [`lane_order`], not the declaration
//! order of [`LaneRole`]; [`ALL_LANES`] lists the roles in rank order, and [`lane_id`] keys
//! off the same rank. The `u32` values in [`role_id`] and [`tex_role_id`] are wire ids the
//! browser API passes in; they never change. The renderer's `LaneId` carries no name: the
//! cartographic identities live here.

/// One named map draw lane; its paint position is [`lane_order`], not declaration order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LaneRole {
    /// Synthetic benchmark quads the stress bench streams; rank 0, under everything.
    Stress,

    /// The two boot calibration quads (green, red), drawn until hidden; rank 0 after `Stress`.
    Calibration,

    /// Basemap texture: satellite or cartographic imagery ([`tex_role_id::BASEMAP`]).
    Satellite,

    /// Sea underlay polygon mesh ([`role_id::SEA`]); world layer key `sea`.
    Sea,

    /// Hillshade texture over the basemap ([`tex_role_id::HILLSHADE`]).
    Hillshade,

    /// land-cover hulls.
    Landcover,

    /// Terrain elevation contour hairlines ([`role_id::CONTOURS`]); world layer key `contours`.
    Contours,

    /// Airfield apron polygon mesh ([`role_id::AIRFIELD_APRON`]); world layer key `airfield`.
    WorldAirfieldApron,

    /// Road casing strip triangles under `Roads` ([`role_id::ROADS_CASING`]); key `roads`.
    RoadsCasing,

    /// Road centreline strip triangles over their casing ([`role_id::ROADS`]); key `roads`.
    Roads,

    /// world-building OBB fills (`world-buildings`).
    WorldBuildings,

    /// world-building outline casing (`world-buildings-outline`).
    WorldBuildingsOutline,

    /// World fence strips from `upload_world_fence_strips`.
    WorldFences,

    /// Forest mass polygon fill ([`role_id::FOREST_FILL`]); world layer key `forest`.
    ForestFill,

    /// Forest mass outline hairlines ([`role_id::FOREST_OUTLINE`]); toggled with the fill.
    ForestOutline,

    /// tree + vegetation glyphs.
    WorldTrees,

    /// prop + rockLarge glyphs.
    WorldProps,

    /// building badges.
    WorldBadges,

    /// Height and cartographic text labels, 20-byte instances; world layer key `heights`.
    WorldLabels,

    /// Road name text labels; world layer key `roadNames`.
    WorldRoadLabels,

    /// Town, village, airport and locality name labels; world layer key `townLabels`.
    WorldTownLabels,

    /// Building interior floor plates, roof cells and stair plates (polygon mesh).
    InteriorSlabs,

    /// Furniture and prop footprints inside a building, coloured by cover tier.
    InteriorFurniture,

    /// Hairline outlines of the interior furniture footprints.
    InteriorFurnitureOutline,

    /// Wall section cuts at eye height as strip triangles.
    InteriorWalls,

    /// Wall section cuts as constant 1-pixel hairlines, with sills and lower-floor cuts.
    InteriorWallsOutline,

    /// Door leaves where they hang, and door frames.
    InteriorPortals,

    /// Door swing arcs as hairlines.
    InteriorPortalsOutline,

    /// Glass pane cuts of a building's windows.
    InteriorGlazing,

    /// Window-frame jamb ticks as hairlines.
    InteriorGlazingOutline,

    /// Stair tread hatch hairlines.
    InteriorStairs,

    /// Tree trunk discs and canopies of the building interior debug scene.
    SceneVegetation,

    /// Tree rim and stipple hairlines of the building interior debug scene.
    SceneVegetationOutline,

    /// Terrain viewshed texture of the line-of-sight tool (`viewshed_upload`).
    Viewshed,

    /// Line-of-sight probe ray, split and coloured at each hit, with event dots.
    InteriorProbe,

    /// Order: above `Grid` (zones are mission data, not world chrome) and below `SquadLinks`, so a zone ring can enclose the units it contains without ever occluding a slot marker.
    MissionZones,

    /// Mission marker glyphs with their captions.
    MissionMarkers,

    /// Mission comment bubble glyphs.
    MissionComments,

    /// Order: above `MissionComments` (an edge is mission data, and it must composite over the annotation glyphs it may pass under) and below `SquadLinks` — the squad hairlines are structural ORBAT truth and win the overprint, exactly as every mission lane loses to `Slots`, so a connection can never occlude a unit ring.
    MissionConnections,

    /// Squad hierarchy hairlines between slots ([`role_id::SQUAD_LINKS`]); under `Slots`.
    SquadLinks,

    /// Mission vehicle symbols: silhouettes, or discs when zoomed out.
    MissionVehicles,

    /// mission slot rings.
    Slots,

    /// The ghost ring that previews where a new slot lands.
    SlotPlacePreview,

    /// Overlay of the slots being dragged, moved by the shader drag offset.
    SlotDrag,

    /// Cluster discs that stand in for slots when many slots are zoomed far out.
    Clusters,

    /// The 1 km map grid lines.
    Grid,

    /// Selection marquee fill (topmost with its outline).
    Marquee,

    /// Selection marquee outline, cleared with `Marquee`.
    MarqueeOutline,
}

/// Paint rank of `role`, 0 (bottom) to 47 (top); `Stress` and `Calibration` share rank 0.
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

/// The renderer's opaque key for a lane.
///
/// The graphics engine sorts ascending on this and never interprets it; the 48 named
/// identities, their order and their wire ids all stay here, where cartography belongs. It
/// reuses [`lane_order`], so the renderer's batch order is the lane rank.
///
/// The doubling is the one wrinkle: [`lane_order`] is a total order but is NOT injective —
/// `Stress` and `Calibration` share rank 0, and the two are distinct lanes the diagnostics
/// tell apart. `2·order` leaves an odd slot beside every rank, and `Calibration` takes its
/// own, which also preserves the order those two have always drawn in (the calibration quads
/// are pushed after the stress chunks and paint over them).
#[must_use]
pub fn lane_id(role: LaneRole) -> render_primitives::frame::ids::LaneId {
    let bump = u16::from(matches!(role, LaneRole::Calibration));
    render_primitives::frame::ids::LaneId(u16::from(lane_order(role)) * 2 + bump)
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

// Public role ids for the **vector-lane upload API** (`upload_polygon_mesh`,
// `upload_strip_tris`, `upload_hairline_segments`, `clear_vector_lane`).
/// Wire ids of the vector lanes the browser upload API addresses; see [`lane_role_from_u32`].
pub mod role_id {

    /// Sea underlay polygon mesh.
    pub const SEA: u32 = 0;

    /// Land-cover hull polygon mesh.
    pub const LANDCOVER: u32 = 1;

    /// DEM contour hairlines.
    pub const CONTOURS: u32 = 2;

    /// Road casing strip triangles.
    pub const ROADS_CASING: u32 = 3;

    /// Road centerline strip triangles.
    pub const ROADS: u32 = 4;

    /// Forest mass polygon mesh.
    pub const FOREST_FILL: u32 = 5;

    /// Forest mass outline hairlines.
    pub const FOREST_OUTLINE: u32 = 6;

    /// Selection marquee fill (drops `MarqueeOutline` with it on `clear_vector_lane`).
    pub const MARQUEE: u32 = 7;

    /// NW Everon airfield apron polygon mesh.
    pub const AIRFIELD_APRON: u32 = 8;

    /// Canonical squad links value.
    pub const SQUAD_LINKS: u32 = 9;

    /// Canonical mission zones value.
    pub const MISSION_ZONES: u32 = 10;

    /// Canonical interior slabs value.
    pub const INTERIOR_SLABS: u32 = 11;

    /// Canonical interior furniture value.
    pub const INTERIOR_FURNITURE: u32 = 12;

    /// Canonical interior furniture outline value.
    pub const INTERIOR_FURNITURE_OUTLINE: u32 = 13;

    /// Canonical interior walls value.
    pub const INTERIOR_WALLS: u32 = 14;

    /// Canonical interior walls outline value.
    pub const INTERIOR_WALLS_OUTLINE: u32 = 15;

    /// Canonical interior portals value.
    pub const INTERIOR_PORTALS: u32 = 16;

    /// Canonical interior portals outline value.
    pub const INTERIOR_PORTALS_OUTLINE: u32 = 17;

    /// Canonical interior glazing value.
    pub const INTERIOR_GLAZING: u32 = 18;

    /// Canonical interior glazing outline value.
    pub const INTERIOR_GLAZING_OUTLINE: u32 = 19;

    /// Canonical interior stairs value.
    pub const INTERIOR_STAIRS: u32 = 20;

    /// Canonical scene vegetation value.
    pub const SCENE_VEGETATION: u32 = 21;

    /// Canonical scene vegetation outline value.
    pub const SCENE_VEGETATION_OUTLINE: u32 = 22;

    /// Canonical interior probe value.
    pub const INTERIOR_PROBE: u32 = 23;

    /// Highest assigned id. `lane_role_from_u32` returns `None` above this.
    pub const MAX: u32 = INTERIOR_PROBE;
}

/// The lane of vector-lane wire id `role` ([`role_id`]); `None` above [`role_id::MAX`].
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

// Public role ids for the **texture-lane API** (`tex_layer_begin` / `tex_layer_commit` /
// `tex_layer_clear` / `set_lane_opacity`). A **disjoint** namespace from [`role_id`]: these
// index a fixed `[Option<PendingTex>; 2]` bucket in the engine, and id `0` means `Satellite`
// here but `Sea` over there.
/// Wire ids of the two texture lanes, disjoint from [`role_id`]; see [`tex_lane_role_from_u32`].
pub mod tex_role_id {

    /// Basemap lane — the satellite/Map cartographic texture (`satellite.rs`'s `ROLE_BASEMAP`).
    pub const BASEMAP: u32 = 0;

    /// Hillshade overlay lane (`world_assets/mod.rs::apply_hillshade`).
    pub const HILLSHADE: u32 = 1;

    /// Highest assigned id. `tex_lane_role_from_u32` returns `None` above this, and the engine's `pending: [Option<PendingTex>; 2]` bucket is exactly `MAX + 1` long.
    pub const MAX: u32 = HILLSHADE;
}

/// Map a texture-lane role u32 → [`LaneRole`]. `None` for anything that is not `0` or `1`.
pub fn tex_lane_role_from_u32(role: u32) -> Option<LaneRole> {
    Some(match role {
        tex_role_id::BASEMAP => LaneRole::Satellite,
        tex_role_id::HILLSHADE => LaneRole::Hillshade,
        _ => return None,
    })
}

#[cfg(test)]
#[path = "tests/draw_order_tests.rs"]
mod lane_order_pins;
