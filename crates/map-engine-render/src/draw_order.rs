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
    /// T-090.11.5 — building bench: floor slabs / plates (polygon mesh). The twelve `Interior*` /
    /// `Scene*` lanes sit above every world lane and label and below `Viewshed`, so an interior
    /// plan composites over the terrain and the wash still paints over it.
    InteriorSlabs,
    /// T-090.11.5 — furniture / prop footprints (polygon mesh, cover-tier colour).
    InteriorFurniture,
    /// T-090.11.5 — furniture footprint outlines (hairlines).
    InteriorFurnitureOutline,
    /// T-090.11.5 — wall section cuts (strip triangles).
    InteriorWalls,
    /// T-090.11.5 — wall-cut hairline twin (never thins out at low zoom), rings and ghosts.
    InteriorWallsOutline,
    /// T-090.11.5 — door leaves and frames (strip triangles).
    InteriorPortals,
    /// T-090.11.5 — door swing arcs (hairlines).
    InteriorPortalsOutline,
    /// T-090.11.5 — glass panes (strip triangles, cyan).
    InteriorGlazing,
    /// T-090.11.5 — window-frame jamb ticks (hairlines).
    InteriorGlazingOutline,
    /// T-090.11.5 — stairs tread hatch (hairlines).
    InteriorStairs,
    /// T-090.11.5 — scene trees: trunk disc + canopy (polygon mesh).
    SceneVegetation,
    /// T-090.11.5 — canopy stipple / outline (hairlines).
    SceneVegetationOutline,
    /// T-644 — Line-of-Sight viewshed wash: a per-cell visible/hidden RGBA raster over the world
    /// rect, uploaded as ONE texture (the forest-density lane's exact shape — own texture + a
    /// 1-instance world-rect quad, NOT the 2-slot `pending` basemap bucket). Sits ABOVE all world
    /// geometry + labels (so the desaturated dead-ground wash composites over contours / landcover /
    /// forest — the T-640 contour hairlines show through its α0.38) and BELOW `Grid` + every mission
    /// lane (so the wash never dims the grid ticks or occludes a slot marker / zone ring). Session-
    /// only, cleared on tool/sub-mode switch; never persisted.
    Viewshed,
    /// T-090.11.5 — the building bench's LOS probe: ray strip + event dots, ABOVE the wash (the
    /// verdict must read over the disc) and BELOW `Grid`.
    InteriorProbe,
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
    /// T-760 — briefing marker glyphs (above zone rings, below squad links + slots so a marker
    /// never occludes a unit). Slot-atlas discs; not on the pick/SoA bridge.
    MissionMarkers,
    /// T-748 — editor-only comment glyphs (above briefing markers, below squad links + slots so a
    /// note never occludes a unit). Slot-atlas rings; not on the pick/SoA bridge (same hazard
    /// T-760 documented for markers — comments must not ride `slots_bind_soa`).
    MissionComments,
    /// T-780 — the editor-only CONNECTION graph, drawn as one hairline segment per edge between
    /// its two endpoints (`Contours` / `ForestOutline` / `SquadLinks` / `MissionZones` shape — a
    /// flat `[x,y,r,g,b,a]…` LineList, no new pipeline and no new atlas).
    ///
    /// **Why this lane exists at all.** T-672 shipped the connection graph with no map artifact, so
    /// `CONN-DEL-001` was reachable only from the Connections panel's per-row Delete: an author who
    /// drew an edge on the map had nothing on the map to click. A line is the artifact.
    ///
    /// Order: above `MissionComments` (an edge is mission data, and it must composite over the
    /// annotation glyphs it may pass under) and below `SquadLinks` — the squad hairlines are
    /// structural ORBAT truth and win the overprint, exactly as every mission lane loses to
    /// `Slots`, so a connection can never occlude a unit ring.
    ///
    /// Fed by its own typed engine API (`connections_bind`), like `MissionMarkers` /
    /// `MissionComments` and unlike `SquadLinks` — so it has no `role_id` and cannot be reached
    /// through the generic vector-lane upload path.
    MissionConnections,
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
        // T-090.11.5: the building bench's twelve interior / scene lanes sit above every world lane
        // and label and below the viewshed wash; the probe sits above the wash. The same sanctioned
        // uniform shift as T-644 / T-592 — pure in-memory sort keys, relations pinned in the tests.
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
        // T-644: the viewshed wash sits above ALL world chrome + labels, below the grid + mission
        // lanes. Inserting here shifts the grid + mission block +1 (the SAME sanctioned uniform shift
        // T-592 did for zones) — these integers are a pure in-memory sort key for `upsert_lane`,
        // never serialized/persisted, so the shift is safe exactly as long as the `lane_order_pins`
        // relations below still hold (they pin RELATIONS, e.g. Viewshed < Grid < Slots, not values).
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

/// Every [`LaneRole`], listed once. `all_lanes_covers_every_variant` makes this impossible to
/// leave stale: adding a variant breaks that test's exhaustive `match` at compile time, and the
/// tag-set assertion then fails until the variant is listed here too. Tests that must consider
/// *every* lane (e.g. `marquee_lanes_are_topmost_fill_then_border`) derive from this rather than
/// a hand-kept copy — a hand-kept copy silently stops examining each new lane.
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
    /// T-090.11.5 building bench — floor slabs / plates polygon mesh.
    pub const INTERIOR_SLABS: u32 = 11;
    /// T-090.11.5 — furniture / prop footprints polygon mesh.
    pub const INTERIOR_FURNITURE: u32 = 12;
    /// T-090.11.5 — furniture footprint outline hairlines.
    pub const INTERIOR_FURNITURE_OUTLINE: u32 = 13;
    /// T-090.11.5 — wall section-cut strip triangles.
    pub const INTERIOR_WALLS: u32 = 14;
    /// T-090.11.5 — wall-cut hairline twin, rings, ghosts.
    pub const INTERIOR_WALLS_OUTLINE: u32 = 15;
    /// T-090.11.5 — door leaves + frames strip triangles.
    pub const INTERIOR_PORTALS: u32 = 16;
    /// T-090.11.5 — door swing-arc hairlines.
    pub const INTERIOR_PORTALS_OUTLINE: u32 = 17;
    /// T-090.11.5 — glass pane strip triangles.
    pub const INTERIOR_GLAZING: u32 = 18;
    /// T-090.11.5 — window-frame jamb-tick hairlines.
    pub const INTERIOR_GLAZING_OUTLINE: u32 = 19;
    /// T-090.11.5 — stairs tread-hatch hairlines.
    pub const INTERIOR_STAIRS: u32 = 20;
    /// T-090.11.5 — scene tree trunk + canopy polygon mesh.
    pub const SCENE_VEGETATION: u32 = 21;
    /// T-090.11.5 — canopy stipple / outline hairlines.
    pub const SCENE_VEGETATION_OUTLINE: u32 = 22;
    /// T-090.11.5 — the LOS probe strip (ray + event dots), above the wash.
    pub const INTERIOR_PROBE: u32 = 23;
    /// Highest assigned id. `lane_role_from_u32` returns `None` above this.
    pub const MAX: u32 = INTERIOR_PROBE;
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
        | LaneRole::MissionMarkers
        | LaneRole::MissionComments
        // T-780: fed by `connections_bind`, the typed API markers/comments use — no upload id, so
        // the generic `upload_hairline_segments(role, …)` path cannot reach this lane by number.
        | LaneRole::MissionConnections
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
#[path = "draw_order_tests.rs"]
mod lane_order_pins;
