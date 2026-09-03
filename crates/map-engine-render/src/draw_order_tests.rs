//! `draw_order` pin tests (`mod lane_order_pins`), split out per the `#[path]` precedent so
//! the lane table itself stays under the SIZE gate (T-090.11.5).

use super::{
    ALL_LANES, LaneRole as L, lane_order, lane_role_from_u32, lane_role_to_u32, role_id,
    tex_lane_role_from_u32, tex_role_id,
};

/// T-592 — the role→u32→role round trip, proved **exhaustive in both directions** rather
/// than spot-checked, over `ALL_LANES` (which cannot go stale).
#[test]
fn wire_round_trip_is_exhaustive_both_ways() {
    // Forward: every lane that has an id round-trips to itself; every lane without one is
    // genuinely unreachable from the upload API.
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
                !(0..=role_id::MAX).any(|i| lane_role_from_u32(i) == Some(role)),
                "{role:?} has no to_u32 id but is reachable from from_u32"
            ),
        }
    }
    // Backward: ids are dense over 0..=MAX and each maps back to itself.
    for id in 0..=role_id::MAX {
        let role = lane_role_from_u32(id)
            .unwrap_or_else(|| panic!("id {id} is a hole in a dense 0..={} range", role_id::MAX));
        assert_eq!(
            lane_role_to_u32(role),
            Some(id),
            "id {id} did not round-trip"
        );
    }
    // The two directions describe the same set — no id-less lane, no lane-less id.
    assert_eq!(with_id, usize::try_from(role_id::MAX).unwrap() + 1);
    assert_eq!(
        with_id, 24,
        "T-090.11.5 added ids 11..=23; 24 vector lanes carry an upload id"
    );
}

/// T-592 / T-244 guard — an unknown id must be an inert `None`, never a panic and never an
/// index into a fixed-size bucket. `engine.rs` `let … else { return; }`s on every one of
/// these, so this is the whole out-of-range contract.
#[test]
fn unknown_role_ids_are_none_not_a_panic() {
    for id in [
        role_id::MAX + 1,
        role_id::MAX + 2,
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

/// T-592 — each id pinned individually, so a renumber has to be deliberate rather than a
/// side effect of editing the `role_id` block.
///
/// T-596 removed the eight frontend hand-copies this originally guarded, so a renumber now
/// *propagates* to the callsites instead of leaving them stale. This test is still the
/// tripwire, but for a different failure: it is the one place a renumber must be typed out
/// twice, which is what stops an accidental one.
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
    // The named constants and the literals above must agree.
    assert_eq!(role_id::SEA, 0);
    assert_eq!(role_id::SQUAD_LINKS, 9);
    assert_eq!(role_id::MISSION_ZONES, 10);
    assert_eq!(role_id::INTERIOR_SLABS, 11);
    assert_eq!(role_id::INTERIOR_PROBE, 23);
    assert_eq!(role_id::MAX, 23);
}

/// T-596 (item 7) — the texture-lane ids, pinned the same way. Disjoint from `role_id`:
/// **id 0 is `Satellite` here and `Sea` there**, which is exactly the confusion the two
/// separate modules exist to prevent.
#[test]
fn tex_wire_ids_are_pinned() {
    assert_eq!(tex_lane_role_from_u32(0), Some(L::Satellite));
    assert_eq!(tex_lane_role_from_u32(1), Some(L::Hillshade));
    assert_eq!(tex_role_id::BASEMAP, 0);
    assert_eq!(tex_role_id::HILLSHADE, 1);
    assert_eq!(tex_role_id::MAX, 1);
    // The two namespaces disagree on 0 by design — pinned so a "helpful" future unification
    // has to delete this line rather than silently merge them.
    assert_eq!(lane_role_from_u32(0), Some(L::Sea));
}

/// T-596 (item 7) — the regression test for `set_lane_opacity`'s old fail-open. Before this
/// slice the engine ran `if role == 0 { Satellite } else { Hillshade }`, so every id below
/// would have resolved to `Hillshade` and re-tinted the wrong lane. `None` is the whole
/// point: it is what lets the caller distinguish "unknown id" from "id 1".
#[test]
fn unknown_tex_role_ids_are_none_not_hillshade() {
    for id in [
        tex_role_id::MAX + 1,
        tex_role_id::MAX + 2,
        role_id::MISSION_ZONES,
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

/// T-180.4: squad leader lines draw above the grid, under slot rings.
#[test]
fn squad_links_sit_between_grid_and_slots() {
    assert!(lane_order(L::SquadLinks) > lane_order(L::Grid));
    assert!(lane_order(L::SquadLinks) < lane_order(L::Slots));
}

/// T-180.8: vehicle discs above squad links, under slot rings.
#[test]
fn mission_vehicles_sit_between_squad_links_and_slots() {
    assert!(lane_order(L::MissionVehicles) > lane_order(L::SquadLinks));
    assert!(lane_order(L::MissionVehicles) < lane_order(L::Slots));
}

/// T-592: derived from `ALL_LANES`, not a hand-kept copy. The previous hand-list would have
/// happily passed while never examining a newly added lane — the signature defect in
/// miniature. `ALL_LANES` cannot go stale (see `all_lanes_covers_every_variant`), so this
/// test now genuinely considers every lane that exists.
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

/// `ALL_LANES` lists every variant exactly once, and cannot silently fall behind the enum:
/// adding a variant breaks the exhaustive `match` below at **compile time**, and the tag-set
/// assertion then fails until the variant is added to `ALL_LANES` and the count bumped.
#[test]
fn all_lanes_covers_every_variant() {
    // Exhaustive, no `_` arm — a new variant is a compile error here.
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

/// T-644: the viewshed wash sits above ALL world chrome + labels (so a dead-ground wash
/// composites over contours / forest / roads / town labels) but below the grid and every mission
/// lane (so it never dims a grid tick or occludes a slot / zone). The RELATIONS the placement
/// rests on — pinned so a future renumber that breaks them fails here, not on-screen.
#[test]
fn viewshed_sits_above_world_chrome_below_grid_and_mission() {
    // Above every world geometry + label lane.
    assert!(lane_order(L::Viewshed) > lane_order(L::Contours));
    assert!(lane_order(L::Viewshed) > lane_order(L::Landcover));
    assert!(lane_order(L::Viewshed) > lane_order(L::ForestFill));
    assert!(lane_order(L::Viewshed) > lane_order(L::ForestOutline));
    assert!(lane_order(L::Viewshed) > lane_order(L::Roads));
    assert!(lane_order(L::Viewshed) > lane_order(L::WorldTrees));
    assert!(lane_order(L::Viewshed) > lane_order(L::WorldTownLabels));
    // Below the grid + every mission lane (never occludes markers/zones, never dims grid ticks).
    assert!(lane_order(L::Viewshed) < lane_order(L::Grid));
    assert!(lane_order(L::Viewshed) < lane_order(L::MissionZones));
    assert!(lane_order(L::Viewshed) < lane_order(L::SquadLinks));
    assert!(lane_order(L::Viewshed) < lane_order(L::Slots));
    assert!(lane_order(L::Viewshed) < lane_order(L::Marquee));
}

/// T-592: zone rings open the mission block — above the grid (mission data outranks world
/// chrome), below every marker lane so a ring never occludes a unit it encloses.
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

/// T-760: briefing markers above zone rings, below squad links + slots (never occlude a unit).
#[test]
fn mission_markers_sit_between_zones_and_squad_links() {
    assert!(lane_order(L::MissionMarkers) > lane_order(L::MissionZones));
    assert!(lane_order(L::MissionMarkers) < lane_order(L::MissionComments));
    assert!(lane_order(L::MissionMarkers) < lane_order(L::SquadLinks));
    assert!(lane_order(L::MissionMarkers) < lane_order(L::MissionVehicles));
    assert!(lane_order(L::MissionMarkers) < lane_order(L::Slots));
}

/// T-748: editor comment glyphs above briefing markers, below squad links + slots.
#[test]
fn mission_comments_sit_between_markers_and_squad_links() {
    assert!(lane_order(L::MissionComments) > lane_order(L::MissionMarkers));
    assert!(lane_order(L::MissionComments) > lane_order(L::MissionZones));
    assert!(lane_order(L::MissionComments) < lane_order(L::SquadLinks));
    assert!(lane_order(L::MissionComments) < lane_order(L::MissionVehicles));
    assert!(lane_order(L::MissionComments) < lane_order(L::Slots));
}

/// T-780: connection edges above the comment/marker glyphs, below squad links + vehicles +
/// slots — an edge never occludes a unit ring, and the ORBAT hairlines win the overprint.
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

/// T-780 — the connection lane is fed by its OWN typed engine API, exactly like
/// `MissionMarkers` / `MissionComments`. It must therefore have NO `role_id`: an upload id
/// would make `upload_hairline_segments(role, …)` a second door into the same lane, and a
/// second door is a second vocabulary. Pinned in both directions.
#[test]
fn mission_connections_has_no_wire_upload_id() {
    assert_eq!(lane_role_to_u32(L::MissionConnections), None);
    for id in 0..=role_id::MAX {
        assert_ne!(
            lane_role_from_u32(id),
            Some(L::MissionConnections),
            "id {id} must not resolve to the typed-API connection lane"
        );
    }
}

#[test]
fn basemap_stack_order_is_deck_parity() {
    // satellite → sea → hillshade → landcover → contours → roads → buildings → forest.
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

/// T-152.4: first role after trees is still WorldProps — compute-cull pin unchanged.
#[test]
fn first_role_after_trees_is_props() {
    assert_eq!(lane_order(L::WorldProps), lane_order(L::WorldTrees) + 1);
}

/// T-175 B2: the palette place-preview ghost draws above placed slots, below the drag overlay.
#[test]
fn place_preview_sits_above_slots_below_drag() {
    assert!(lane_order(L::SlotPlacePreview) > lane_order(L::Slots));
    assert!(lane_order(L::SlotPlacePreview) < lane_order(L::SlotDrag));
    assert!(lane_order(L::SlotPlacePreview) < lane_order(L::Marquee));
}

/// T-748 — `mission_history` is wasm32-only, so its feed cannot host a native Class-R pin here
/// as a same-crate module. Pin both live call sites via `include_str!` so deleting either
/// `comments_bind` feed turns RED (lane-order pins never examine the feeder).
#[cfg(test)]
mod t748_comments_bind_feed {
    const HIST: &str = include_str!("../../../apps/website/frontend/src/editor/state/history.rs");

    fn only_body(src: &str, sig: &str) -> String {
        let start = src
            .find(sig)
            .unwrap_or_else(|| panic!("missing signature: {sig}"));
        let after = &src[start..];
        let brace = after.find('{').expect("missing body");
        let mut depth = 0usize;
        let mut end = brace;
        for (i, ch) in after[brace..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = brace + i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        after[..end].to_string()
    }

    fn comments_bind_needle() -> String {
        format!("{}{}", "comments", "_bind")
    }

    fn comment_lane_xy_needle() -> String {
        format!("{}{}", "comment_lane_", "xy")
    }

    #[test]
    fn rebind_and_after_doc_change_both_feed_comments_bind() {
        let rebind = only_body(HIST, "pub fn rebind_engine_from_doc");
        let after = only_body(HIST, "fn after_doc_change");
        let bind = comments_bind_needle();
        let pack = comment_lane_xy_needle();
        assert!(
            rebind.contains(&bind),
            "T-748: rebind_engine_from_doc must call comments_bind; body:\n{rebind}"
        );
        assert!(
            rebind.contains(&pack),
            "T-748: rebind_engine_from_doc must pack via comment_lane_xy; body:\n{rebind}"
        );
        assert!(
            after.contains(&bind),
            "T-748: after_doc_change must call comments_bind; body:\n{after}"
        );
        assert!(
            after.contains(&pack),
            "T-748: after_doc_change must pack via comment_lane_xy; body:\n{after}"
        );
    }
}

/// T-748 — `engine.rs` is wasm32-gated, so its body cannot host a native Class-R pin.
/// Source-inspect `comments_bind` here: must upload `MissionComments` and must not touch
/// the pick bridge (`last_ids` / `slots_bind_soa`).
#[cfg(test)]
mod t748_comments_bind_pick_bridge {
    const ENGINE: &str = include_str!("engine.rs");

    fn comments_bind_body() -> String {
        let sig = "pub fn comments_bind";
        let start = ENGINE
            .find(sig)
            .unwrap_or_else(|| panic!("T-748: missing {sig}"));
        let after = &ENGINE[start..];
        let brace = after.find('{').expect("comments_bind body");
        let mut depth = 0usize;
        let mut end = brace;
        for (i, ch) in after[brace..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = brace + i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        after[..end].to_string()
    }

    #[test]
    fn comments_bind_body_does_not_touch_last_ids() {
        let body = comments_bind_body();
        assert!(
            !body.contains("last_ids"),
            "T-748: comments_bind must not touch pick-bridge last_ids; body:\n{body}"
        );
        assert!(
            body.contains("MissionComments"),
            "T-748: comments_bind must upload LaneRole::MissionComments; body:\n{body}"
        );
        assert!(
            !body.contains("slots_bind_soa"),
            "T-748: comments_bind must not call slots_bind_soa; body:\n{body}"
        );
    }
}

/// T-780 — same shape as the T-748 pin one module up: `engine.rs` is wasm32-gated, so
/// `connections_bind`'s body cannot host a native Class-R pin of its own. Source-inspect it here.
///
/// The body is extracted by brace-matching from its signature, so this module's own source is not
/// in the haystack — the needles cannot be satisfied by the assertion text that searches for them.
#[cfg(test)]
mod t780_connections_bind_pick_bridge {
    const ENGINE: &str = include_str!("engine.rs");

    fn connections_bind_body() -> String {
        let sig = "pub fn connections_bind";
        let start = ENGINE
            .find(sig)
            .unwrap_or_else(|| panic!("T-780: missing {sig}"));
        let after = &ENGINE[start..];
        let brace = after.find('{').expect("connections_bind body");
        let mut depth = 0usize;
        let mut end = brace;
        for (i, ch) in after[brace..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = brace + i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        after[..end].to_string()
    }

    /// The connection lane is a RENDER lane, not a pick surface: the map hit-test runs app-side off
    /// the same document rows that feed the lane (`mission_editor::pick_connection`), so this body
    /// must never enter the slot pick / SoA bridge — the identical hazard T-760 and T-748 pin.
    #[test]
    fn connections_bind_body_uploads_its_lane_and_skips_the_pick_bridge() {
        let body = connections_bind_body();
        assert!(
            body.contains("MissionConnections"),
            "T-780: connections_bind must upload LaneRole::MissionConnections; body:\n{body}"
        );
        assert!(
            !body.contains("last_ids"),
            "T-780: connections_bind must not touch pick-bridge last_ids; body:\n{body}"
        );
        assert!(
            !body.contains("slots_bind_soa"),
            "T-780: connections_bind must not call slots_bind_soa; body:\n{body}"
        );
    }
}

/// T-808 — the four paths the symbology work added to `engine.rs`, pinned here for the same reason
/// T-748 and T-780 are: `engine.rs` is `#[cfg(target_arch = "wasm32")]`, so it hosts no native test
/// of its own and `cargo test -p map-engine-render` never compiles a line of it. Before this module
/// the new paths had NO pin at all — `ensure_slot_atlas` could stop widening the atlas,
/// `slots_bind_symbology` could stop keeping the role/heading columns, `vehicles_bind_symbology`
/// could lose its no-symbology fallback and `refresh_comment_lane` could stop being called, and the
/// whole suite would stay green because none of it is native code.
///
/// SCRUBBED SOURCE, not raw `include_str!` self-matching: the haystack is another file
/// (`engine.rs`) narrowed to ONE function body by brace-matching from its signature, so this
/// module's own assertion text is never in the string being searched and a needle cannot be
/// satisfied by the test that looks for it — the T-759 hollow-pin class, twice caught this week.
/// One extractor serves four tests rather than four copies of the brace matcher above.
#[cfg(test)]
mod t808_symbology_bind_paths {
    const ENGINE: &str = include_str!("engine.rs");

    /// Brace-matched body of the unique `sig` in `engine.rs`. Panics if the signature is missing
    /// (a renamed / deleted path is a red pin, never a silently skipped one) or ambiguous.
    fn body(sig: &str) -> String {
        let start = ENGINE
            .find(sig)
            .unwrap_or_else(|| panic!("T-808: engine.rs has no `{sig}`"));
        assert!(
            !ENGINE[start + sig.len()..].contains(sig),
            "T-808: `{sig}` is not unique in engine.rs — the extractor would pin the wrong body"
        );
        let after = &ENGINE[start..];
        let brace = after.find('{').expect("body");
        let mut depth = 0usize;
        let mut end = brace;
        for (i, ch) in after[brace..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = brace + i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        after[..end].to_string()
    }

    /// The atlas is widened ENGINE-side, so every one of the three call sites that build a strip
    /// gets the symbology cells without knowing they exist. Both arms are load-bearing: the `Some`
    /// arm must record the runtime base, and the `None` arm must record `None` rather than leave a
    /// stale base pointing into an atlas that no longer has those cells (the lanes read
    /// `symbology_base` to decide between a symbology glyph id and the pre-T-808 ring).
    #[test]
    fn ensure_slot_atlas_widens_and_records_the_symbology_base() {
        let b = body("pub fn ensure_slot_atlas");
        assert!(
            b.contains("extend_atlas_with_unit_glyphs"),
            "T-808: ensure_slot_atlas must widen the strip via \
             slots_gpu::extend_atlas_with_unit_glyphs; body:\n{b}"
        );
        assert!(
            b.contains("symbology_base = Some(wide.base_cells)"),
            "T-808: ensure_slot_atlas must record the runtime symbology base; body:\n{b}"
        );
        assert!(
            b.contains("symbology_base = None"),
            "T-808: an unparseable strip must CLEAR symbology_base, not keep a stale one; \
             body:\n{b}"
        );
        assert!(
            b.contains("atlas_ready = true"),
            "T-808: ensure_slot_atlas must still arm the slot bridge; body:\n{b}"
        );
    }

    /// The symbology bind is the only writer of the two columns the map was missing. Losing either
    /// store is silent — every slot just draws as the rifleman default facing north, which is
    /// exactly the pre-T-808 look this ticket exists to replace.
    #[test]
    fn slots_bind_symbology_keeps_the_role_and_heading_columns() {
        let b = body("pub fn slots_bind_symbology");
        assert!(
            b.contains("last_roles = roles"),
            "T-808: slots_bind_symbology must store the ROLE column; body:\n{b}"
        );
        assert!(
            b.contains("last_headings = headings_deg"),
            "T-808: slots_bind_symbology must store the HEADING column; body:\n{b}"
        );
        assert!(
            b.contains("rematerialize_slot_lane"),
            "T-808: slots_bind_symbology must re-pack the slot lane from the new columns; \
             body:\n{b}"
        );
        // The pre-T-808 entry point must keep delegating here, or every caller that was never
        // updated silently stops carrying symbology at all.
        let soa = body("pub fn slots_bind_soa");
        assert!(
            soa.contains("slots_bind_symbology"),
            "T-808: slots_bind_soa must delegate to slots_bind_symbology; body:\n{soa}"
        );
    }

    /// The vehicle lane packs SILHOUETTES from the symbology cells, and must degrade to
    /// `vehicles_bind`'s disc lane when the uploaded atlas has none — emitting symbology glyph ids
    /// against a two-cell atlas samples whatever the shader clamp lands on. Like every other
    /// MissionVehicles path it is a RENDER lane, never a pick surface (the T-748 / T-780 hazard).
    #[test]
    fn vehicles_bind_symbology_uploads_its_lane_and_falls_back_without_symbology() {
        let b = body("pub fn vehicles_bind_symbology");
        assert!(
            b.contains("MissionVehicles"),
            "T-808: vehicles_bind_symbology must upload LaneRole::MissionVehicles; body:\n{b}"
        );
        assert!(
            b.contains("pack_vehicle_symbology"),
            "T-808: vehicles_bind_symbology must pack via slots_gpu::pack_vehicle_symbology; \
             body:\n{b}"
        );
        assert!(
            b.contains("symbology_base") && b.contains("self.vehicles_bind(xy)"),
            "T-808: vehicles_bind_symbology must fall back to the disc lane when the atlas \
             carries no symbology cells; body:\n{b}"
        );
        assert!(
            !b.contains("last_ids") && !b.contains("slots_bind_soa"),
            "T-808: vehicles_bind_symbology must not enter the slot pick / SoA bridge; body:\n{b}"
        );
    }

    /// The comment lane carries no row identity inside the engine, so selection and the symbology
    /// zoom crossing can only reach it by re-packing from the cached `comment_xy`. Pin the body AND
    /// its two feeds: a private fn that nothing calls is a hollow pin.
    #[test]
    fn refresh_comment_lane_repacks_from_cache_and_is_actually_called() {
        let b = body("fn refresh_comment_lane(&mut self)");
        assert!(
            b.contains("comment_xy"),
            "T-808: refresh_comment_lane must re-pack from the cached comment_xy; body:\n{b}"
        );
        assert!(
            b.contains("self.comments_bind(&xy)"),
            "T-808: refresh_comment_lane must re-pack through comments_bind; body:\n{b}"
        );
        for feed in ["pub fn set_selection", "fn sync_slot_zoom_uniform"] {
            let f = body(feed);
            assert!(
                f.contains("refresh_comment_lane"),
                "T-808: `{feed}` must refresh the comment lane; body:\n{f}"
            );
        }
    }
}

/// T-090.11.5: the building bench's twelve interior / scene lanes sit above every world lane and
/// label (an interior plan composites over the terrain) and below the viewshed wash (the wash
/// still paints over the plan), in the listed order with every fill below its outline twin.
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

/// T-090.11.5: the LOS probe reads over the wash disc but never over the grid or a mission lane.
#[test]
fn probe_sits_between_viewshed_and_grid() {
    assert!(lane_order(L::InteriorProbe) > lane_order(L::Viewshed));
    assert!(lane_order(L::InteriorProbe) > lane_order(L::SceneVegetationOutline));
    assert!(lane_order(L::InteriorProbe) < lane_order(L::Grid));
    assert!(lane_order(L::InteriorProbe) < lane_order(L::MissionZones));
    assert_eq!(
        lane_role_to_u32(L::InteriorProbe),
        Some(role_id::INTERIOR_PROBE)
    );
}
