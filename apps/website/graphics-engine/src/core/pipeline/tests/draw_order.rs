//! Role: draw order.
//! Position: `core/pipeline/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::pipeline::draw_order::ALL_LANES;
use crate::core::pipeline::draw_order::LaneRole as L;
use crate::core::pipeline::draw_order::lane_order;
use crate::core::pipeline::draw_order::lane_role_from_u32;
use crate::core::pipeline::draw_order::lane_role_to_u32;

use crate::core::pipeline::draw_order::tex_lane_role_from_u32;

#[test]
fn wire_round_trip_is_exhaustive_both_ways() {
    let mut with_id = 0;
    for role in ALL_LANES {
        match lane_role_to_u32(role) {
            Some(id) => {
                assert_eq!(
                    lane_role_from_u32(id),
                    Some(role),
                    "role {role:?} → id {id} did not round-trip"
                );
                with_id += 1;
            }
            None => assert!(
                !(0..=crate::core::pipeline::draw_order::role_id::MAX)
                    .any(|i| lane_role_from_u32(i) == Some(role)),
                "{role:?} has no to_u32 id but is reachable from from_u32"
            ),
        }
    }

    for id in 0..=crate::core::pipeline::draw_order::role_id::MAX {
        let role = lane_role_from_u32(id).unwrap_or_else(|| {
            panic!(
                "id {id} is a hole in a dense 0..={} range",
                crate::core::pipeline::draw_order::role_id::MAX
            )
        });
        assert_eq!(
            lane_role_to_u32(role),
            Some(id),
            "id {id} did not round-trip"
        );
    }

    assert_eq!(
        with_id,
        usize::try_from(crate::core::pipeline::draw_order::role_id::MAX).unwrap() + 1
    );
    assert_eq!(
        with_id, 24,
        "T-090.11.5 added ids 11..=23; 24 vector lanes carry an upload id"
    );
}

#[test]
fn unknown_role_ids_are_none_not_a_panic() {
    for id in [
        crate::core::pipeline::draw_order::role_id::MAX + 1,
        crate::core::pipeline::draw_order::role_id::MAX + 2,
        24,
        99,
        256,
        65_536,
        u32::MAX - 1,
        u32::MAX,
    ] {
        assert_eq!(lane_role_from_u32(id), None, "id {id} must be unknown");
    }
}

#[test]
fn wire_ids_are_pinned() {
    for (id, role) in [
        (0, L::Sea),
        (1, L::Landcover),
        (2, L::Contours),
        (3, L::RoadsCasing),
        (4, L::Roads),
        (5, L::ForestFill),
        (6, L::ForestOutline),
        (7, L::Marquee),
        (8, L::WorldAirfieldApron),
        (9, L::SquadLinks),
        (10, L::MissionZones),
        (11, L::InteriorSlabs),
        (12, L::InteriorFurniture),
        (13, L::InteriorFurnitureOutline),
        (14, L::InteriorWalls),
        (15, L::InteriorWallsOutline),
        (16, L::InteriorPortals),
        (17, L::InteriorPortalsOutline),
        (18, L::InteriorGlazing),
        (19, L::InteriorGlazingOutline),
        (20, L::InteriorStairs),
        (21, L::SceneVegetation),
        (22, L::SceneVegetationOutline),
        (23, L::InteriorProbe),
    ] {
        assert_eq!(lane_role_from_u32(id), Some(role), "id {id} moved");
    }

    assert_eq!(crate::core::pipeline::draw_order::role_id::SEA, 0);
    assert_eq!(crate::core::pipeline::draw_order::role_id::SQUAD_LINKS, 9);
    assert_eq!(
        crate::core::pipeline::draw_order::role_id::MISSION_ZONES,
        10
    );
    assert_eq!(
        crate::core::pipeline::draw_order::role_id::INTERIOR_SLABS,
        11
    );
    assert_eq!(
        crate::core::pipeline::draw_order::role_id::INTERIOR_PROBE,
        23
    );
    assert_eq!(crate::core::pipeline::draw_order::role_id::MAX, 23);
}

#[test]
fn tex_wire_ids_are_pinned() {
    assert_eq!(tex_lane_role_from_u32(0), Some(L::Satellite));
    assert_eq!(tex_lane_role_from_u32(1), Some(L::Hillshade));
    assert_eq!(crate::core::pipeline::draw_order::tex_role_id::BASEMAP, 0);
    assert_eq!(crate::core::pipeline::draw_order::tex_role_id::HILLSHADE, 1);
    assert_eq!(crate::core::pipeline::draw_order::tex_role_id::MAX, 1);

    assert_eq!(lane_role_from_u32(0), Some(L::Sea));
}

#[test]
fn unknown_tex_role_ids_are_none_not_hillshade() {
    for id in [
        crate::core::pipeline::draw_order::tex_role_id::MAX + 1,
        crate::core::pipeline::draw_order::tex_role_id::MAX + 2,
        crate::core::pipeline::draw_order::role_id::MISSION_ZONES,
        7,
        99,
        u32::MAX,
    ] {
        assert_eq!(
            tex_lane_role_from_u32(id),
            None,
            "tex id {id} must be unknown, not silently Hillshade"
        );
    }
}

#[test]
fn airfield_apron_sits_between_contours_and_roads() {
    assert!(lane_order(L::WorldAirfieldApron) > lane_order(L::Contours));
    assert!(lane_order(L::WorldAirfieldApron) < lane_order(L::RoadsCasing));
    assert!(lane_order(L::RoadsCasing) < lane_order(L::Roads));
}

#[test]
fn fences_sit_between_building_outline_and_forest() {
    assert!(lane_order(L::WorldFences) > lane_order(L::WorldBuildingsOutline));
    assert!(lane_order(L::WorldFences) < lane_order(L::WorldBadges));
    assert!(lane_order(L::WorldFences) < lane_order(L::WorldTrees));
}

#[test]
fn labels_sit_between_badges_and_grid() {
    assert!(lane_order(L::WorldLabels) > lane_order(L::WorldBadges));
    assert!(lane_order(L::WorldRoadLabels) > lane_order(L::WorldLabels));
    assert!(lane_order(L::WorldRoadLabels) > lane_order(L::Roads));
    assert!(lane_order(L::WorldTownLabels) > lane_order(L::WorldRoadLabels));
    assert!(lane_order(L::WorldTownLabels) < lane_order(L::Grid));
}

#[test]
fn grid_sits_between_world_glyphs_and_mission_lanes() {
    assert!(lane_order(L::Grid) > lane_order(L::WorldBadges));
    assert!(lane_order(L::Grid) > lane_order(L::WorldLabels));
    assert!(lane_order(L::Grid) > lane_order(L::WorldRoadLabels));
    assert!(lane_order(L::Grid) > lane_order(L::WorldTownLabels));
    assert!(lane_order(L::Grid) > lane_order(L::WorldTrees));
    assert!(lane_order(L::Grid) > lane_order(L::WorldProps));
    assert!(lane_order(L::Grid) < lane_order(L::SquadLinks));
    assert!(lane_order(L::Grid) < lane_order(L::Slots));
    assert!(lane_order(L::Grid) < lane_order(L::SlotDrag));
    assert!(lane_order(L::Grid) < lane_order(L::Clusters));
}

#[test]
fn squad_links_sit_between_grid_and_slots() {
    assert!(lane_order(L::SquadLinks) > lane_order(L::Grid));
    assert!(lane_order(L::SquadLinks) < lane_order(L::Slots));
}

#[test]
fn mission_vehicles_sit_between_squad_links_and_slots() {
    assert!(lane_order(L::MissionVehicles) > lane_order(L::SquadLinks));
    assert!(lane_order(L::MissionVehicles) < lane_order(L::Slots));
}

#[test]
fn marquee_lanes_are_topmost_fill_then_border() {
    let max_non_marquee = ALL_LANES
        .into_iter()
        .filter(|r| !matches!(r, L::Marquee | L::MarqueeOutline))
        .map(lane_order)
        .max()
        .unwrap();
    assert!(lane_order(L::Marquee) > max_non_marquee);
    assert!(lane_order(L::MarqueeOutline) > lane_order(L::Marquee));
}

#[test]
fn all_lanes_covers_every_variant() {
    fn tag(r: L) -> u8 {
        match r {
            L::Stress => 0,
            L::Calibration => 1,
            L::Satellite => 2,
            L::Sea => 3,
            L::Hillshade => 4,
            L::Landcover => 5,
            L::Contours => 6,
            L::WorldAirfieldApron => 7,
            L::RoadsCasing => 8,
            L::Roads => 9,
            L::WorldBuildings => 10,
            L::WorldBuildingsOutline => 11,
            L::WorldFences => 12,
            L::ForestFill => 13,
            L::ForestOutline => 14,
            L::WorldTrees => 15,
            L::WorldProps => 16,
            L::WorldBadges => 17,
            L::WorldLabels => 18,
            L::WorldRoadLabels => 19,
            L::WorldTownLabels => 20,
            L::InteriorSlabs => 21,
            L::InteriorFurniture => 22,
            L::InteriorFurnitureOutline => 23,
            L::InteriorWalls => 24,
            L::InteriorWallsOutline => 25,
            L::InteriorPortals => 26,
            L::InteriorPortalsOutline => 27,
            L::InteriorGlazing => 28,
            L::InteriorGlazingOutline => 29,
            L::InteriorStairs => 30,
            L::SceneVegetation => 31,
            L::SceneVegetationOutline => 32,
            L::Viewshed => 33,
            L::InteriorProbe => 34,
            L::Grid => 35,
            L::MissionZones => 36,
            L::MissionMarkers => 37,
            L::MissionComments => 38,
            L::MissionConnections => 39,
            L::SquadLinks => 40,
            L::MissionVehicles => 41,
            L::Slots => 42,
            L::SlotPlacePreview => 43,
            L::SlotDrag => 44,
            L::Clusters => 45,
            L::Marquee => 46,
            L::MarqueeOutline => 47,
        }
    }
    let mut tags: Vec<u8> = ALL_LANES.into_iter().map(tag).collect();
    tags.sort_unstable();
    let expected: Vec<u8> = (0..48).collect();
    assert_eq!(
        tags, expected,
        "ALL_LANES must list every variant exactly once"
    );
}

#[test]
fn viewshed_sits_above_world_chrome_below_grid_and_mission() {
    assert!(lane_order(L::Viewshed) > lane_order(L::Contours));
    assert!(lane_order(L::Viewshed) > lane_order(L::Landcover));
    assert!(lane_order(L::Viewshed) > lane_order(L::ForestFill));
    assert!(lane_order(L::Viewshed) > lane_order(L::ForestOutline));
    assert!(lane_order(L::Viewshed) > lane_order(L::Roads));
    assert!(lane_order(L::Viewshed) > lane_order(L::WorldTrees));
    assert!(lane_order(L::Viewshed) > lane_order(L::WorldTownLabels));

    assert!(lane_order(L::Viewshed) < lane_order(L::Grid));
    assert!(lane_order(L::Viewshed) < lane_order(L::MissionZones));
    assert!(lane_order(L::Viewshed) < lane_order(L::SquadLinks));
    assert!(lane_order(L::Viewshed) < lane_order(L::Slots));
    assert!(lane_order(L::Viewshed) < lane_order(L::Marquee));
}

#[test]
fn mission_zones_sit_between_grid_and_squad_links() {
    assert!(lane_order(L::MissionZones) > lane_order(L::Grid));
    assert!(lane_order(L::MissionZones) < lane_order(L::MissionMarkers));
    assert!(lane_order(L::MissionZones) < lane_order(L::MissionComments));
    assert!(lane_order(L::MissionZones) < lane_order(L::SquadLinks));
    assert!(lane_order(L::MissionZones) < lane_order(L::MissionVehicles));
    assert!(lane_order(L::MissionZones) < lane_order(L::Slots));
    assert!(lane_order(L::MissionZones) < lane_order(L::Marquee));
}

#[test]
fn mission_markers_sit_between_zones_and_squad_links() {
    assert!(lane_order(L::MissionMarkers) > lane_order(L::MissionZones));
    assert!(lane_order(L::MissionMarkers) < lane_order(L::MissionComments));
    assert!(lane_order(L::MissionMarkers) < lane_order(L::SquadLinks));
    assert!(lane_order(L::MissionMarkers) < lane_order(L::MissionVehicles));
    assert!(lane_order(L::MissionMarkers) < lane_order(L::Slots));
}

#[test]
fn mission_comments_sit_between_markers_and_squad_links() {
    assert!(lane_order(L::MissionComments) > lane_order(L::MissionMarkers));
    assert!(lane_order(L::MissionComments) > lane_order(L::MissionZones));
    assert!(lane_order(L::MissionComments) < lane_order(L::SquadLinks));
    assert!(lane_order(L::MissionComments) < lane_order(L::MissionVehicles));
    assert!(lane_order(L::MissionComments) < lane_order(L::Slots));
}

#[test]
fn mission_connections_sit_between_comments_and_squad_links() {
    assert!(lane_order(L::MissionConnections) > lane_order(L::MissionComments));
    assert!(lane_order(L::MissionConnections) > lane_order(L::MissionMarkers));
    assert!(lane_order(L::MissionConnections) > lane_order(L::MissionZones));
    assert!(lane_order(L::MissionConnections) > lane_order(L::Grid));
    assert!(lane_order(L::MissionConnections) < lane_order(L::SquadLinks));
    assert!(lane_order(L::MissionConnections) < lane_order(L::MissionVehicles));
    assert!(lane_order(L::MissionConnections) < lane_order(L::Slots));
}

#[test]
fn mission_connections_has_no_wire_upload_id() {
    assert_eq!(lane_role_to_u32(L::MissionConnections), None);
    for id in 0..=crate::core::pipeline::draw_order::role_id::MAX {
        assert_ne!(
            lane_role_from_u32(id),
            Some(L::MissionConnections),
            "id {id} must not resolve to the typed-API connection lane"
        );
    }
}

#[test]
fn basemap_stack_order_is_deck_parity() {
    let chain = [
        L::Satellite,
        L::Sea,
        L::Hillshade,
        L::Landcover,
        L::Contours,
        L::WorldAirfieldApron,
        L::RoadsCasing,
        L::Roads,
        L::WorldBuildings,
        L::WorldBuildingsOutline,
        L::WorldFences,
        L::ForestFill,
        L::ForestOutline,
        L::WorldTrees,
        L::WorldProps,
        L::WorldBadges,
    ];
    for w in chain.windows(2) {
        assert!(
            lane_order(w[0]) < lane_order(w[1]),
            "order violated: {:?} !< {:?}",
            lane_order(w[0]),
            lane_order(w[1])
        );
    }
}

#[test]
fn first_role_after_trees_is_props() {
    assert_eq!(lane_order(L::WorldProps), lane_order(L::WorldTrees) + 1);
}

#[test]
fn place_preview_sits_above_slots_below_drag() {
    assert!(lane_order(L::SlotPlacePreview) > lane_order(L::Slots));
    assert!(lane_order(L::SlotPlacePreview) < lane_order(L::SlotDrag));
    assert!(lane_order(L::SlotPlacePreview) < lane_order(L::Marquee));
}

#[cfg(test)]
#[path = "tests/draw_order_t748_comments_bind_feed.rs"]
mod t748_comments_bind_feed;

#[cfg(test)]
#[path = "tests/draw_order_t748_comments_bind_pick_bridge.rs"]
mod t748_comments_bind_pick_bridge;

#[cfg(test)]
#[path = "tests/draw_order_t780_connections_bind_pick_bridge.rs"]
mod t780_connections_bind_pick_bridge;

#[cfg(test)]
#[path = "tests/draw_order_t808_symbology_bind_paths.rs"]
mod t808_symbology_bind_paths;

#[test]
fn interior_lanes_sit_between_town_labels_and_viewshed() {
    let ordered = [
        L::InteriorSlabs,
        L::InteriorFurniture,
        L::InteriorFurnitureOutline,
        L::InteriorWalls,
        L::InteriorWallsOutline,
        L::InteriorPortals,
        L::InteriorPortalsOutline,
        L::InteriorGlazing,
        L::InteriorGlazingOutline,
        L::InteriorStairs,
        L::SceneVegetation,
        L::SceneVegetationOutline,
    ];
    for lane in ordered {
        assert!(
            lane_order(lane) > lane_order(L::WorldTownLabels),
            "{lane:?}"
        );
        assert!(lane_order(lane) > lane_order(L::WorldTrees), "{lane:?}");
        assert!(lane_order(lane) < lane_order(L::Viewshed), "{lane:?}");
        assert!(lane_order(lane) < lane_order(L::Grid), "{lane:?}");
        assert_eq!(
            lane_role_from_u32(lane_role_to_u32(lane).expect("upload id")),
            Some(lane),
            "{lane:?} must be reachable from the upload API"
        );
    }
    for w in ordered.windows(2) {
        assert!(
            lane_order(w[0]) < lane_order(w[1]),
            "{:?} < {:?}",
            w[0],
            w[1]
        );
    }
    for (fill, outline) in [
        (L::InteriorFurniture, L::InteriorFurnitureOutline),
        (L::InteriorWalls, L::InteriorWallsOutline),
        (L::InteriorPortals, L::InteriorPortalsOutline),
        (L::InteriorGlazing, L::InteriorGlazingOutline),
        (L::SceneVegetation, L::SceneVegetationOutline),
    ] {
        assert!(lane_order(fill) < lane_order(outline));
    }
}

#[test]
fn probe_sits_between_viewshed_and_grid() {
    assert!(lane_order(L::InteriorProbe) > lane_order(L::Viewshed));
    assert!(lane_order(L::InteriorProbe) > lane_order(L::SceneVegetationOutline));
    assert!(lane_order(L::InteriorProbe) < lane_order(L::Grid));
    assert!(lane_order(L::InteriorProbe) < lane_order(L::MissionZones));
    assert_eq!(
        lane_role_to_u32(L::InteriorProbe),
        Some(crate::core::pipeline::draw_order::role_id::INTERIOR_PROBE)
    );
}
