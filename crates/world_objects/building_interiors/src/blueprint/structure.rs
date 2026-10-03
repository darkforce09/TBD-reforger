//! A building blueprint: its levels and the walls, doors, windows, stairs and furniture on
//! them.
//!
//! **Role:** the camelCase blueprint JSON model ([`BuildingBlueprint`], [`BuildingLevel`] and the
//! element records).
//! **Position:** read from `prefabs/buildings/<slug>.json` or rebuilt from the archive by
//! [`crate::blueprint::archive`]; the attribution, the section drawings, the map engine's interior
//! line of sight and the developer tools' blueprint pipeline read it.
//! **Signals & state:** none; plain data.
//! **Invariants:** the JSON round-trips byte-for-byte through the typed identifiers; a door or
//! window names the wall it sits in by its [`WallId`].

use crate::blueprint::footprint::OverallFootprint;
use crate::blueprint::footprint::PlateGrid;
use crate::blueprint::footprint::RoofGrid;
use crate::blueprint::footprint::VerticalProfile;
use serde::Deserialize;
use serde::Serialize;
use world_file_formats::ids::{DoorId, FurnitureId, StairsId, WallId, WindowId};

use crate::building_ids::BuildingPrefabId;

/// 2D Bounding box representation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BBox2D {
    /// Low corner `[x, z]` of the box in the building's local frame, metres.
    pub min: [f64; 2],

    /// High corner `[x, z]` of the box in the building's local frame, metres.
    pub max: [f64; 2],

    /// Extent along local X (`max[0] - min[0]`), metres; JSON `widthM`.
    pub width_m: f64,

    /// Extent along local Z (`max[1] - min[1]`), metres; JSON `depthM`.
    pub depth_m: f64,
}

/// Wall segment with physical thickness and penetration characteristics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingWall {
    /// Identifier unique within the blueprint; doors and windows name their wall by it.
    pub id: WallId,

    /// First endpoint `[x, z]` of the wall's centre line in the local frame, metres.
    pub start: [f64; 2],

    /// Second endpoint `[x, z]` of the wall's centre line in the local frame, metres.
    pub end: [f64; 2],

    /// Thickness across the centre line, metres (at least 0).
    pub thickness: f64,

    /// Whether the wall is part of the outer shell rather than an interior partition.
    pub is_exterior: bool,

    /// Free-form material name (`wood_log`, `wood_siding`; the scan pipeline writes `scanned`).
    pub material: String,
}

/// Door portal with hinge, swing arc, and glass status.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingDoor {
    /// Identifier unique within the blueprint; sight-line events name the door by it.
    pub id: DoorId,

    /// Enfusion prefab resource path of the door; empty when rebuilt from the archive.
    pub prefab_resource: String,

    /// Identifier of the [`BuildingWall`] the door sits in.
    pub wall_id: WallId,

    /// Centre `[x, z]` of the opening on the wall line, local frame, metres; JSON `pos2D`.
    pub pos2_d: [f64; 2],

    /// Opening width along the wall, metres; a ray within half of it plus 0.05 m slack of
    /// `pos2_d` passes the door.
    pub width_m: f64,

    /// Opening height above the level's floor (`elevation_range[0]`), metres.
    pub height_m: f64,

    /// Hinge side: `left`, `right`, `center` or `none`; the building viewer draws the
    /// swing arc from it. Empty when rebuilt from the archive.
    pub hinge_side: String,

    /// Swing: `inward`, `outward`, `sliding`, `both` or `none`; empty when rebuilt from
    /// the archive.
    pub swing_direction: String,

    /// Whether the door sits in the outer shell, leading outside.
    pub is_exterior: bool,

    /// Whether the door leaf carries glass.
    pub has_glass: bool,

    /// Initial state: `open`, `closed`, `locked` or `destroyed`; only `open` lets a sight
    /// line pass. Empty when rebuilt from the archive.
    pub default_state: String,
}

/// Window opening with sill elevation, aperture dimensions, facing normal, and glass panes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingWindow {
    /// Identifier unique within the blueprint; sight-line events name the window by it.
    pub id: WindowId,

    /// Enfusion prefab resource path of the window; empty when rebuilt from the archive.
    pub prefab_resource: String,

    /// Identifier of the [`BuildingWall`] the window sits in.
    pub wall_id: WallId,

    /// Centre `[x, z]` of the opening on the wall line, local frame, metres; JSON `pos2D`.
    pub pos2_d: [f64; 2],

    /// Opening width along the wall, metres; a ray within half of it plus 0.05 m slack of
    /// `pos2_d` passes the window.
    pub width_m: f64,

    /// Height of the opening's bottom edge above the level's floor, metres.
    pub sill_height_m: f64,

    /// Height of the opening from the sill to its top edge, metres.
    pub window_height_m: f64,

    /// Unit plan direction `[x, z]` the window faces; the building viewer draws a tick
    /// along it.
    pub normal: [f64; 2],

    /// Horizontal field of view through the opening, degrees (0 to 360).
    pub fov_deg: f64,

    /// Whether the opening is glazed.
    pub has_glass: bool,

    /// Number of glass panes in the opening; 0 when rebuilt from the archive.
    pub glass_pane_count: u32,
}

/// Staircase with vertical connection, steps, and tread transparency.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingStairs {
    /// Identifier unique within the blueprint; sight-line events name the stairs by it.
    pub id: StairsId,

    /// Plan rectangle as its low `[x, z]` and high `[x, z]` corners, local frame, metres.
    pub bounds: [[f64; 2]; 2],

    /// `level_index` of the [`BuildingLevel`] the flight leads to.
    pub connects_to_level: usize,

    /// Heading of the flight in the plan, degrees.
    pub direction_deg: f64,

    /// Number of steps (at least 1); the building viewer hatches 4 to 24 treads from it.
    pub step_count: u32,

    /// Whether the treads are open, so a sight line crossing the flight sees through
    /// with `los_concealment`; closed flights add no crossing event.
    pub transparent_steps: bool,

    /// Concealment from 0 (open) to 1 (blocked) a sight line takes on crossing open
    /// treads.
    pub los_concealment: f64,
}

/// Interior furniture prop with 2D bounds, height, and ballistic/visual cover classification.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingFurniture {
    /// Identifier unique within the blueprint; sight-line events name the prop by it.
    pub id: FurnitureId,

    /// Human-readable name of the prop; the scan pipeline falls back to `prop` or
    /// `scanned mass`.
    pub name: String,

    /// Prop category: `table`, `chair`, `bed`, `storage`, `seating`, `appliance`, `crate`
    /// or `prop`.
    pub category: String,

    /// Enfusion prefab resource path; empty for scanned masses and archive rebuilds.
    pub prefab_resource: String,

    /// Centre `[x, z]` of the prop in the local frame, metres; JSON `pos2D`.
    pub pos2_d: [f64; 2],

    /// Yaw relative to the building, degrees; the building viewer turns the plan
    /// rectangle by it, the sight line ignores it.
    pub rotation_deg: f64,

    /// Plan extent `[x, z]` in metres, centred on `pos2_d`; `[0, 0]` when rebuilt from
    /// the archive. JSON `size2D`.
    pub size2_d: [f64; 2],

    /// Top of the prop above the level's floor, metres; a ray above it passes over.
    pub height_m: f64,

    /// Whether the prop obstructs movement.
    pub blocks_movement: bool,

    /// Sight-line cover: `none` (ignored), `low_cover` (0.6 concealment) or `full_cover`
    /// (ends the ray).
    pub los_cover: String,
}

/// A single architectural floor/level of the building.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingLevel {
    /// Zero-based position of the level from the bottom; stairs name levels by it.
    pub level_index: usize,

    /// Display name (`Ground Floor`, `2nd Floor`, `Attic`); empty when rebuilt from the
    /// archive.
    pub name: String,

    /// Floor and ceiling heights `[low, high]` in the local frame (y up), metres; the
    /// band is half-open except on the topmost level.
    pub elevation_range: [f64; 2],

    /// Local height of the level's horizontal plan cut, metres; 0 when rebuilt from the
    /// archive.
    pub slice_height_m: f64,

    /// Largest outer ring of the traced floor plate (empty for slab-less levels like the attic). The full multi-piece truth lives in `floor_polygons` / `plate`.
    pub footprint_polygon: Vec<[f64; 2]>,

    /// Verbatim per-cell floor occupancy + heights; absent on pre-plate blueprints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plate: Option<PlateGrid>,

    /// Traced plate boundary rings (outer + holes per connected piece); empty when untraced.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub floor_polygons: Vec<FloorPolygon>,

    /// Wall segments of this level; its doors and windows name them by id.
    pub walls: Vec<BuildingWall>,

    /// Doors of this level, each sitting in one of `walls`.
    pub doors: Vec<BuildingDoor>,

    /// Windows of this level, each sitting in one of `walls`.
    pub windows: Vec<BuildingWindow>,

    /// Stair flights on this level, each naming the level it leads to.
    pub stairs: Vec<BuildingStairs>,

    /// Props on this level that give cover or block movement.
    pub furniture: Vec<BuildingFurniture>,
}

/// One connected piece of a level's floor plate as traced polygon rings: an outer boundary (CCW) plus any interior voids (CW holes). A level with disconnected floor (split mezzanine) carries several of these.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FloorPolygon {
    /// Outer boundary ring, counter-clockwise, as `[x, z]` points in the local frame,
    /// metres.
    pub outer: Vec<[f64; 2]>,

    /// Interior void rings, clockwise; omitted from the JSON when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holes: Vec<Vec<[f64; 2]>>,
}

/// Complete building archetype blueprint definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingBlueprint {
    /// Blueprint schema version (`1.0.0`); empty when rebuilt from the archive.
    pub schema_version: String,

    /// Building prefab identifier, the blueprint file's slug (`FarmHouse_E_1L01_Green`).
    pub prefab_id: BuildingPrefabId,

    /// Enfusion prefab resource path (`{GUID}Prefabs/...et`); empty when rebuilt from
    /// the archive.
    pub resource_name: String,

    /// Enfusion `.xob` mesh resource path of the building, when known.
    pub model_mesh: Option<String>,

    /// Human-readable building name, when present.
    pub label: Option<String>,

    /// Object kind; the schema allows only `building`. Empty when rebuilt from the archive.
    pub kind: String,

    /// Use category: `residential`, `military`, `commercial`, `industrial`, `civic`,
    /// `sheds_garages` or `generic`. Empty when rebuilt from the archive.
    pub category: String,

    /// Whether the building can be destroyed in game.
    pub destructible: bool,

    /// Foundation, eave, ridge and chimney heights ([`VerticalProfile`]).
    pub vertical_profile: VerticalProfile,

    /// Plan outline, bounding box and area of the building ([`OverallFootprint`]).
    pub overall_footprint: OverallFootprint,

    /// Roof top-surface height grid ([`RoofGrid`]); when absent, no structural hit is
    /// attributed to the roof.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roof: Option<RoofGrid>,

    /// Levels from bottom to top; the last is the topmost, whose band is closed.
    pub levels: Vec<BuildingLevel>,
}
