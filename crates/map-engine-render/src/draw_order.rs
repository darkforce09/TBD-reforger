//! Draw-order contract for the render engine — **pure** (no wgpu/web types) so the ordering
//! relations are natively unit-tested (`lane_order_pins`), while the wasm32-gated `engine`
//! module consumes it (T-151.11.1; audits P-01/X-01).

/// A batch's role — governs the fixed W1 draw order (basemap → hillshade → grid) via
/// [`lane_order`] and lets the editor find/replace a lane in place on LOD / opacity change.
/// `Stress`/`Calibration` are the T-151.0 spike batches (never mixed with the editor lanes).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LaneRole {
    Stress,
    Calibration,
    Satellite,
    /// W4 sea underlay (after basemap, before hillshade).
    Sea,
    Hillshade,
    /// W4 land-cover hulls.
    Landcover,
    Contours,
    /// T-152.5 NW Everon airfield DEM-flat apron (`world-airfield-apron`).
    WorldAirfieldApron,
    RoadsCasing,
    Roads,
    /// W3 world-building OBB fills (`world-buildings`).
    WorldBuildings,
    /// W3 world-building outline casing (`world-buildings-outline`).
    WorldBuildingsOutline,
    /// T-152.4 fence + pier thin strips (`world-fences`).
    WorldFences,
    ForestFill,
    ForestOutline,
    /// W5 tree + vegetation glyphs.
    WorldTrees,
    /// W5 prop + rockLarge glyphs.
    WorldProps,
    /// W5 building badges.
    WorldBadges,
    /// T-152.7 height / ASL text labels (after badges).
    WorldLabels,
    /// T-152.9 road name labels (above roads stroke, below town labels).
    WorldRoadLabels,
    /// T-152.8 town name labels (above road + height labels, below grid).
    WorldTownLabels,
    /// T-644 — Line-of-Sight viewshed wash: a per-cell visible/hidden RGBA raster over the world
    /// rect, uploaded as ONE texture (the forest-density lane's exact shape — own texture + a
    /// 1-instance world-rect quad, NOT the 2-slot `pending` basemap bucket). Sits ABOVE all world
    /// geometry + labels (so the desaturated dead-ground wash composites over contours / landcover /
    /// forest — the T-640 contour hairlines show through its α0.38) and BELOW `Grid` + every mission
    /// lane (so the wash never dims the grid ticks or occludes a slot marker / zone ring). Session-
    /// only, cleared on tool/sub-mode switch; never persisted.
    Viewshed,
    /// T-592 — mission zone rings (under every other mission lane).
    ///
    /// **One lane carries both zone shapes.** A circle and a polygon differ only in how the
    /// *author* edits them; on the GPU both reduce to the same primitive — a closed hairline
    /// loop. `map-engine-core` tessellates a circle into an N-gon ring and a polygon into its
    /// edge loop, and both arrive as one flat `[x,y,r,g,b,a]…` LineList upload, exactly like
    /// `Contours` / `ForestOutline` / `SquadLinks`. Two lanes would buy two GPU buffers, two
    /// upload round-trips and a tie-break rule for overlapping circle-vs-polygon rings that has
    /// no answer (they are peers), in exchange for nothing — so the lane budget spends **one**.
    ///
    /// Order: above `Grid` (zones are mission data, not world chrome) and below `SquadLinks`,
    /// so a zone ring can enclose the units it contains without ever occluding a slot marker.
    MissionZones,
    /// T-180.4 — squad leader→member hairline links (under slot rings).
    SquadLinks,
    /// T-180.8 — mission vehicle discs (under slot rings; pick-safe, separate from Slots).
    MissionVehicles,
    /// W6 mission slot rings.
    Slots,
    /// T-175 B2 — palette place-preview ghost (single translucent ring under the cursor while
    /// dragging an asset from the palette onto the map; above slots, below the drag overlay).
    SlotPlacePreview,
    /// W6 drag-preview overlay (T-061).
    SlotDrag,
    /// W6 cluster discs (T-065).
    Clusters,
    Grid,
    /// Selection marquee fill (topmost with its outline).
    Marquee,
    /// Selection marquee 1 px border (T-151.11.1 — Deck `useSelectionLayer` LINE parity).
    MarqueeOutline,
}

/// Draw-order key (T-151.11.1 — Deck layer-array parity, `c4831451^:TacticalMap.tsx:382-395`):
/// … world glyph lanes → **grid** → slots → slot-drag → clusters → marquee (fill, then border).
/// Deck drew the grid above every world layer but **below** the mission lanes; T-151.6..T-151.10
/// had Grid above Slots/Clusters (grid lines overprinted unit markers — audit P-01).
/// Spike batches sort first, never interleaved. Relations pinned by `lane_order_pins` below.
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
        // T-644: the viewshed wash sits above ALL world chrome + labels, below the grid + mission
        // lanes. Inserting here shifts the grid + mission block +1 (the SAME sanctioned uniform shift
        // T-592 did for zones) — these integers are a pure in-memory sort key for `upsert_lane`,
        // never serialized/persisted, so the shift is safe exactly as long as the `lane_order_pins`
        // relations below still hold (they pin RELATIONS, e.g. Viewshed < Grid < Slots, not values).
        LaneRole::Viewshed => 21,
        LaneRole::Grid => 22,
        LaneRole::MissionZones => 23,
        LaneRole::SquadLinks => 24,
        LaneRole::MissionVehicles => 25,
        LaneRole::Slots => 26,
        LaneRole::SlotPlacePreview => 27,
        LaneRole::SlotDrag => 28,
        LaneRole::Clusters => 29,
        LaneRole::Marquee => 30,
        LaneRole::MarqueeOutline => 31,
    }
}

/// Every [`LaneRole`], listed once. `all_lanes_covers_every_variant` makes this impossible to
/// leave stale: adding a variant breaks that test's exhaustive `match` at compile time, and the
/// tag-set assertion then fails until the variant is listed here too. Tests that must consider
/// *every* lane (e.g. `marquee_lanes_are_topmost_fill_then_border`) derive from this rather than
/// a hand-kept copy — a hand-kept copy silently stops examining each new lane.
pub const ALL_LANES: [LaneRole; 32] = [
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
    LaneRole::Viewshed,
    LaneRole::Grid,
    LaneRole::MissionZones,
    LaneRole::SquadLinks,
    LaneRole::MissionVehicles,
    LaneRole::Slots,
    LaneRole::SlotPlacePreview,
    LaneRole::SlotDrag,
    LaneRole::Clusters,
    LaneRole::Marquee,
    LaneRole::MarqueeOutline,
];

/// Public role ids for the **vector-lane upload API** (`upload_polygon_mesh`,
/// `upload_strip_tris`, `upload_hairline_segments`, `clear_vector_lane`).
///
/// **These are NOT a persisted wire format.** `map-engine-render` is a path dependency of
/// `website-frontend`, so every caller is Rust compiled into the same wasm binary in the same
/// cargo invocation, and no role id is ever written to disk, to the network, or into a mission
/// document. Appending an id is therefore purely additive and renumbering could not break a
/// stored artifact.
///
/// **No hand-written JS calls these** — but the sweep that proves it needs stating precisely.
/// T-592 recorded "zero hits for the upload fns across `*.js` / `*.ts` / `*.html`". There ARE
/// `*.js` hits: T-596 re-ran it and all four fns appear in the **gitignored, wasm-bindgen-
/// GENERATED** glue under `apps/website/frontend/dist/` and `dist-debug/`, where `role` is a
/// passthrough parameter forwarded straight to `wasm.renderengine_*` and never a hardcoded id.
/// Zero hits in `*.ts`, `*.html`, or any tracked source. **The conclusion is unchanged** — role
/// ids are not a wire value — but a future reader who greps and finds the dist glue should be
/// able to see it was already accounted for rather than reopen a settled question.
///
/// The hazard this module closes is **drift, not a panic**: a caller that hand-copies the
/// integer into a private `const ROLE_*: u32` has no compile-time link back to this mapping, so
/// a renumber silently uploads to the wrong lane rather than failing to build. T-592 counted
/// **eight such copies across four frontend files**; **T-596 deleted all eight** — every
/// `upload_*` / `clear_vector_lane` callsite in `website-frontend` now names a `role_id::*`
/// constant directly (`world_assets/dem_vectors.rs`, `world_assets/world_host.rs`,
/// `world_assets/forest_mass.rs`, `mission_history.rs`). **Keep it that way:** import the
/// constant, never re-copy the integer. Measured on the pre-T-596 tree, renaming a constant here
/// left `cargo check -p website-frontend --target wasm32-unknown-unknown` GREEN with eight stale
/// literals behind it; after the change the same rename fails the build at every callsite.
///
/// Not to be confused with the **texture-lane** role ids — see [`tex_role_id`] and
/// [`tex_lane_role_from_u32`]. That is a disjoint namespace over a fixed
/// `[Option<PendingTex>; 2]` bucket; nothing here indexes it, and id `0` means `Sea` on this
/// side but `Satellite` on that one.
///
/// Ids are dense `0..=MAX` and pinned one-by-one by `wire_ids_are_pinned`.
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
    /// T-180.4 squad leader→member hairlines.
    pub const SQUAD_LINKS: u32 = 9;
    /// T-592 mission zone rings — circles and polygons alike, as closed hairline loops.
    pub const MISSION_ZONES: u32 = 10;
    /// Highest assigned id. `lane_role_from_u32` returns `None` above this.
    pub const MAX: u32 = MISSION_ZONES;
}

/// Map a public role u32 (upload API) → [`LaneRole`]. Returns `None` for unknown ids — the four
/// `engine.rs` callsites all `let … else { return; }` on it, so an unknown id is an inert no-op
/// and never an index into a fixed-size bucket. (The T-244 `.expect("kind bucket")` shape does
/// exist in this engine — `tex_layer_begin`'s `pending: [Option<PendingTex>; 2]` — but that is a
/// **disjoint** texture-lane namespace guarded by its own `idx > 1` check, and nothing here
/// indexes it.)
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
        _ => return None,
    })
}

/// Inverse of [`lane_role_from_u32`]. `None` for the engine-internal lanes that have no upload
/// id (spike batches, textured lanes, residency-composed world lanes, the mission lanes fed by
/// their own typed APIs). Exists so the round-trip can be proved exhaustive in both directions
/// rather than spot-checked — see `wire_round_trip_is_exhaustive_both_ways`.
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
        // No upload id — reached only through a typed engine API or the spike path.
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
        // T-644: the viewshed lane is fed by its OWN typed engine API (`viewshed_upload` /
        // `viewshed_clear`, the forest-density shape), not the generic vector-lane upload-id path, so
        // it has no `role_id` — exactly like `ForestFill`'s companion typed lanes and the world lanes.
        | LaneRole::Viewshed
        | LaneRole::Grid
        | LaneRole::MissionVehicles
        | LaneRole::Slots
        | LaneRole::SlotPlacePreview
        | LaneRole::SlotDrag
        | LaneRole::Clusters
        | LaneRole::MarqueeOutline => return None,
    })
}

/// Public role ids for the **texture-lane API** (`tex_layer_begin` / `tex_layer_commit` /
/// `tex_layer_clear` / `set_lane_opacity`). A **disjoint** namespace from [`role_id`]: these
/// index a fixed `[Option<PendingTex>; 2]` bucket in the engine, and id `0` means `Satellite`
/// here but `Sea` over there.
///
/// Two ids, and there is no third — the bucket is a fixed-size array, so `MAX` is a property of
/// the engine's storage rather than a list that can grow by appending.
pub mod tex_role_id {
    /// Basemap lane — the satellite/Map cartographic texture (`satellite.rs`'s `ROLE_BASEMAP`).
    pub const BASEMAP: u32 = 0;
    /// Hillshade overlay lane (`world_assets/mod.rs::apply_hillshade`).
    pub const HILLSHADE: u32 = 1;
    /// Highest assigned id. `tex_lane_role_from_u32` returns `None` above this, and the engine's
    /// `pending: [Option<PendingTex>; 2]` bucket is exactly `MAX + 1` long.
    pub const MAX: u32 = HILLSHADE;
}

/// Map a texture-lane role u32 → [`LaneRole`]. `None` for anything that is not `0` or `1`.
///
/// **T-596 (diagnosis item 7) — this exists to close a fail-open.** `engine.rs::set_lane_opacity`
/// resolved its role with `if role == 0 { Satellite } else { Hillshade }`, so **every** id from
/// `1` to `u32::MAX` collapsed onto Hillshade — while `tex_layer_begin`, two functions away,
/// rejected `idx > 1` explicitly. Latent, because the live callers only ever pass `0`
/// (`satellite.rs::show_satellite_basemap`) and `1` (`world_assets/mod.rs`, twice); the defect
/// was that the *next* caller would be silently misrouted instead of corrected.
///
/// Living here rather than in `engine.rs` is deliberate: `engine` is `#[cfg(target_arch =
/// "wasm32")]` and `RenderEngine` needs a real GPU, so a guard written inline there **cannot be
/// unit-tested at all**. `draw_order` compiles natively, so the decision is provable under plain
/// `cargo test` — see `tex_wire_ids_are_pinned` / `unknown_tex_role_ids_are_none_not_hillshade`.
pub fn tex_lane_role_from_u32(role: u32) -> Option<LaneRole> {
    Some(match role {
        tex_role_id::BASEMAP => LaneRole::Satellite,
        tex_role_id::HILLSHADE => LaneRole::Hillshade,
        _ => return None,
    })
}

/// T-151.11.1 — lane-order pins (audit P-01/X-01). These relations ARE the layer contract;
/// any renumbering that breaks Deck parity fails here before it can ship.
#[cfg(test)]
mod lane_order_pins {
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
            let role = lane_role_from_u32(id).unwrap_or_else(|| {
                panic!("id {id} is a hole in a dense 0..={} range", role_id::MAX)
            });
            assert_eq!(
                lane_role_to_u32(role),
                Some(id),
                "id {id} did not round-trip"
            );
        }
        // The two directions describe the same set — no id-less lane, no lane-less id.
        assert_eq!(with_id, usize::try_from(role_id::MAX).unwrap() + 1);
        assert_eq!(
            with_id, 11,
            "T-592 added id 10; 11 vector lanes carry an upload id"
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
            12,
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
        ] {
            assert_eq!(lane_role_from_u32(id), Some(role), "id {id} moved");
        }
        // The named constants and the literals above must agree.
        assert_eq!(role_id::SEA, 0);
        assert_eq!(role_id::SQUAD_LINKS, 9);
        assert_eq!(role_id::MISSION_ZONES, 10);
        assert_eq!(role_id::MAX, 10);
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
                L::Viewshed => 21,
                L::Grid => 22,
                L::MissionZones => 23,
                L::SquadLinks => 24,
                L::MissionVehicles => 25,
                L::Slots => 26,
                L::SlotPlacePreview => 27,
                L::SlotDrag => 28,
                L::Clusters => 29,
                L::Marquee => 30,
                L::MarqueeOutline => 31,
            }
        }
        let mut tags: Vec<u8> = ALL_LANES.into_iter().map(tag).collect();
        tags.sort_unstable();
        let expected: Vec<u8> = (0..32).collect();
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
        assert!(lane_order(L::MissionZones) < lane_order(L::SquadLinks));
        assert!(lane_order(L::MissionZones) < lane_order(L::MissionVehicles));
        assert!(lane_order(L::MissionZones) < lane_order(L::Slots));
        assert!(lane_order(L::MissionZones) < lane_order(L::Marquee));
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
}
