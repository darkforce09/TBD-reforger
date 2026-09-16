//! Role: Module boundary for architecture/blueprint/model.
//! Position: `world/architecture/blueprint/model` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Tests.
#[cfg(test)]
#[path = "../tests/model.rs"]
pub(crate) mod tests;

/// Re-export `crate::world::architecture::blueprint::structure::BBox2D`.
pub use crate::world::architecture::blueprint::structure::BBox2D;

/// Re-export `crate::world::architecture::blueprint::structure::BuildingBlueprint`.
pub use crate::world::architecture::blueprint::structure::BuildingBlueprint;

/// Re-export `crate::world::architecture::blueprint::structure::BuildingDoor`.
pub use crate::world::architecture::blueprint::structure::BuildingDoor;

/// Re-export `crate::world::architecture::blueprint::structure::BuildingFurniture`.
pub use crate::world::architecture::blueprint::structure::BuildingFurniture;

/// Re-export `crate::world::architecture::blueprint::structure::BuildingLevel`.
pub use crate::world::architecture::blueprint::structure::BuildingLevel;

/// Re-export `crate::world::architecture::blueprint::structure::BuildingStairs`.
pub use crate::world::architecture::blueprint::structure::BuildingStairs;

/// Re-export `crate::world::architecture::blueprint::structure::BuildingWall`.
pub use crate::world::architecture::blueprint::structure::BuildingWall;

/// Re-export `crate::world::architecture::blueprint::structure::BuildingWindow`.
pub use crate::world::architecture::blueprint::structure::BuildingWindow;

/// Re-export `crate::world::architecture::blueprint::structure::FloorPolygon`.
pub use crate::world::architecture::blueprint::structure::FloorPolygon;

/// Re-export `crate::world::architecture::blueprint::footprint::OverallFootprint`.
pub use crate::world::architecture::blueprint::footprint::OverallFootprint;

/// Re-export `crate::world::architecture::blueprint::footprint::PlateGrid`.
pub use crate::world::architecture::blueprint::footprint::PlateGrid;

/// Re-export `crate::world::architecture::blueprint::footprint::RoofGrid`.
pub use crate::world::architecture::blueprint::footprint::RoofGrid;

/// Re-export `crate::world::architecture::blueprint::footprint::VerticalProfile`.
pub use crate::world::architecture::blueprint::footprint::VerticalProfile;

/// Re-export `crate::world::architecture::blueprint::attribution_1::LosHit`.
pub use crate::world::architecture::blueprint::attribution_1::LosHit;

/// Re-export `crate::world::architecture::blueprint::attribution_1::LosHitKind`.
pub use crate::world::architecture::blueprint::attribution_1::LosHitKind;

/// Re-export `crate::world::architecture::blueprint::attribution_1::LosResult`.
pub use crate::world::architecture::blueprint::attribution_1::LosResult;

/// Re-export `crate::world::architecture::blueprint::attribution_1::clip_t_to_band`.
pub use crate::world::architecture::blueprint::attribution_1::clip_t_to_band;

/// Re-export `crate::world::architecture::blueprint::geometry::dist_2d`.
pub use crate::world::architecture::blueprint::geometry::dist_2d;

/// Re-export `crate::world::architecture::blueprint::geometry::line_segment_intersection_2d`.
pub use crate::world::architecture::blueprint::geometry::line_segment_intersection_2d;

/// Re-export `crate::world::architecture::blueprint::geometry::segment_intersects_aabb_2d`.
pub use crate::world::architecture::blueprint::geometry::segment_intersects_aabb_2d;
