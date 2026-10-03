//! The names a reader of a building's interior imports with `use building_interiors::prelude::*;`.

pub use crate::blueprint::footprint::{OverallFootprint, PlateGrid, RoofGrid, VerticalProfile};
pub use crate::blueprint::sight_line::{LosHit, LosHitKind, LosResult, clip_t_to_band};
pub use crate::blueprint::structure::{
    BBox2D, BuildingBlueprint, BuildingDoor, BuildingFurniture, BuildingLevel, BuildingStairs,
    BuildingWall, BuildingWindow, FloorPolygon,
};
pub use crate::building_ids::{BuildingFeatureId, BuildingPrefabId, CompoundInstanceId};
pub use crate::compound::assembly::{
    CompoundBuilding, CompoundError, CoverTier, FlatMesh, PlacementSource,
};
pub use crate::compound::doors::{DoorRecord, DoorState};
pub use crate::compound::instances::{
    Instance, InstanceKind, InstanceRecord, InstancesFile, LocalTransform, instances_from_records,
};
pub use crate::error::{Error, Result};
pub use crate::section::cutter::{
    BuildingDrawing, HeightField, LevelDrawing, LevelSpec, building_drawing, drawing_for,
    section_at, section_at_owned,
};
pub use crate::section::index::{SparseHeights, YIntervalIndex, triangles_overlapping_y};
