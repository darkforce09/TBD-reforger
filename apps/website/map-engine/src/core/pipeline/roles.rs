//! Role: roles.
//! Position: `core/pipeline` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Role id.
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

/// Tex role id.
pub mod tex_role_id {

    /// Basemap lane — the satellite/Map cartographic texture (`satellite.rs`'s `ROLE_BASEMAP`).
    pub const BASEMAP: u32 = 0;

    /// Hillshade overlay lane (`world_assets/mod.rs::apply_hillshade`).
    pub const HILLSHADE: u32 = 1;

    /// Highest assigned id. `tex_lane_role_from_u32` returns `None` above this, and the engine's `pending: [Option<PendingTex>; 2]` bucket is exactly `MAX + 1` long.
    pub const MAX: u32 = HILLSHADE;
}
