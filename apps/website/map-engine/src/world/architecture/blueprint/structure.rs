//! Role: structure.
//! Position: `world/architecture/blueprint` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::architecture::blueprint::footprint::OverallFootprint;
use crate::world::architecture::blueprint::footprint::PlateGrid;
use crate::world::architecture::blueprint::footprint::RoofGrid;
use crate::world::architecture::blueprint::footprint::VerticalProfile;
use serde::Deserialize;
use serde::Serialize;

/// 2D Bounding box representation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BBox2D {
    /// Min.
    pub min: [f64; 2],

    /// Max.
    pub max: [f64; 2],

    /// Width m.
    pub width_m: f64,

    /// Depth m.
    pub depth_m: f64,
}

/// Wall segment with physical thickness and penetration characteristics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingWall {
    /// Id.
    pub id: String,

    /// Start.
    pub start: [f64; 2],

    /// End.
    pub end: [f64; 2],

    /// Thickness.
    pub thickness: f64,

    /// Is exterior.
    pub is_exterior: bool,

    /// Material.
    pub material: String,
}

/// Door portal with hinge, swing arc, and glass status.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingDoor {
    /// Id.
    pub id: String,

    /// Prefab resource.
    pub prefab_resource: String,

    /// Wall id.
    pub wall_id: String,

    /// Pos2 d.
    pub pos2_d: [f64; 2],

    /// Width m.
    pub width_m: f64,

    /// Height m.
    pub height_m: f64,

    /// Hinge side.
    pub hinge_side: String,

    /// Swing direction.
    pub swing_direction: String,

    /// Is exterior.
    pub is_exterior: bool,

    /// Has glass.
    pub has_glass: bool,

    /// Default state.
    pub default_state: String,
}

/// Window opening with sill elevation, aperture dimensions, facing normal, and glass panes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingWindow {
    /// Id.
    pub id: String,

    /// Prefab resource.
    pub prefab_resource: String,

    /// Wall id.
    pub wall_id: String,

    /// Pos2 d.
    pub pos2_d: [f64; 2],

    /// Width m.
    pub width_m: f64,

    /// Sill height m.
    pub sill_height_m: f64,

    /// Window height m.
    pub window_height_m: f64,

    /// Normal.
    pub normal: [f64; 2],

    /// Fov deg.
    pub fov_deg: f64,

    /// Has glass.
    pub has_glass: bool,

    /// Glass pane count.
    pub glass_pane_count: u32,
}

/// Staircase with vertical connection, steps, and tread transparency.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingStairs {
    /// Id.
    pub id: String,

    /// Bounds.
    pub bounds: [[f64; 2]; 2],

    /// Connects to level.
    pub connects_to_level: usize,

    /// Direction deg.
    pub direction_deg: f64,

    /// Step count.
    pub step_count: u32,

    /// Transparent steps.
    pub transparent_steps: bool,

    /// Los concealment.
    pub los_concealment: f64,
}

/// Interior furniture prop with 2D bounds, height, and ballistic/visual cover classification.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingFurniture {
    /// Id.
    pub id: String,

    /// Name.
    pub name: String,

    /// Category.
    pub category: String,

    /// Prefab resource.
    pub prefab_resource: String,

    /// Pos2 d.
    pub pos2_d: [f64; 2],

    /// Rotation deg.
    pub rotation_deg: f64,

    /// Size2 d.
    pub size2_d: [f64; 2],

    /// Height m.
    pub height_m: f64,

    /// Blocks movement.
    pub blocks_movement: bool,

    /// Los cover.
    pub los_cover: String,
}

/// A single architectural floor/level of the building.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingLevel {
    /// Level index.
    pub level_index: usize,

    /// Name.
    pub name: String,

    /// Elevation range.
    pub elevation_range: [f64; 2],

    /// Slice height m.
    pub slice_height_m: f64,

    /// Largest outer ring of the traced floor plate (empty for slab-less levels like the attic). The full multi-piece truth lives in `floor_polygons` / `plate`.
    pub footprint_polygon: Vec<[f64; 2]>,

    /// Verbatim per-cell floor occupancy + heights; absent on pre-plate blueprints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plate: Option<PlateGrid>,

    /// Traced plate boundary rings (outer + holes per connected piece); empty when untraced.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub floor_polygons: Vec<FloorPolygon>,

    /// Walls.
    pub walls: Vec<BuildingWall>,

    /// Doors.
    pub doors: Vec<BuildingDoor>,

    /// Windows.
    pub windows: Vec<BuildingWindow>,

    /// Stairs.
    pub stairs: Vec<BuildingStairs>,

    /// Furniture.
    pub furniture: Vec<BuildingFurniture>,
}

/// One connected piece of a level's floor plate as traced polygon rings: an outer boundary (CCW) plus any interior voids (CW holes). A level with disconnected floor (split mezzanine) carries several of these.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FloorPolygon {
    /// Outer.
    pub outer: Vec<[f64; 2]>,

    /// Holes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holes: Vec<Vec<[f64; 2]>>,
}

/// Complete building archetype blueprint definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingBlueprint {
    /// Schema version.
    pub schema_version: String,

    /// Prefab id.
    pub prefab_id: String,

    /// Resource name.
    pub resource_name: String,

    /// Model mesh.
    pub model_mesh: Option<String>,

    /// Label.
    pub label: Option<String>,

    /// Kind.
    pub kind: String,

    /// Category.
    pub category: String,

    /// Destructible.
    pub destructible: bool,

    /// Vertical profile.
    pub vertical_profile: VerticalProfile,

    /// Overall footprint.
    pub overall_footprint: OverallFootprint,

    /// Roof.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roof: Option<RoofGrid>,

    /// Levels.
    pub levels: Vec<BuildingLevel>,
}
