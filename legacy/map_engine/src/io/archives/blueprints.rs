//! Role: blueprints.
//! Position: `io/archives` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Blas entry.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct BlasEntry {
    /// Path.
    pub path: String,

    /// Bytes.
    pub bytes: u64,

    /// Tris.
    pub tris: u32,

    /// Kinds.
    pub kinds: [u32; 3],
}

/// Occluder descriptor.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct OccluderDescriptor {
    /// Prefab id.
    pub prefab_id: u32,

    /// Slug.
    pub slug: String,

    /// Kind.
    pub kind: String,

    /// Blocks.
    pub blocks: bool,

    /// Canopy.
    pub canopy: bool,

    /// Local bounds.
    pub local_bounds: [[f32; 3]; 2],

    /// Blas.
    pub blas: Vec<u32>,
}

/// Vertical profile.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct VerticalProfile {
    /// Pivot elevation offset m.
    pub pivot_elevation_offset_m: f32,

    /// Foundation skirt depth m.
    pub foundation_skirt_depth_m: f32,

    /// Total height m.
    pub total_height_m: f32,

    /// Eave height m.
    pub eave_height_m: f32,

    /// Ridge height m.
    pub ridge_height_m: f32,

    /// Roof type.
    pub roof_type: String,
}

/// Wall rec.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct WallRec {
    /// Id.
    pub id: String,

    /// Start.
    pub start: [f32; 2],

    /// End.
    pub end: [f32; 2],

    /// Thickness m.
    pub thickness_m: f32,

    /// Is exterior.
    pub is_exterior: bool,

    /// Material.
    pub material: String,
}

/// Door rec.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct DoorRec {
    /// Id.
    pub id: String,

    /// Wall id.
    pub wall_id: String,

    /// Position.
    pub position: [f32; 2],

    /// Width m.
    pub width_m: f32,

    /// Height m.
    pub height_m: f32,

    /// Is exterior.
    pub is_exterior: bool,

    /// Has glass.
    pub has_glass: bool,
}

/// Window rec.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct WindowRec {
    /// Id.
    pub id: String,

    /// Wall id.
    pub wall_id: String,

    /// Position.
    pub position: [f32; 2],

    /// Width m.
    pub width_m: f32,

    /// Sill height m.
    pub sill_height_m: f32,

    /// Window height m.
    pub window_height_m: f32,

    /// Normal.
    pub normal: [f32; 2],

    /// Fov deg.
    pub fov_deg: f32,

    /// Has glass.
    pub has_glass: bool,
}

/// Stairs rec.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct StairsRec {
    /// Id.
    pub id: String,

    /// Bounds.
    pub bounds: [[f32; 2]; 2],

    /// Connects to level.
    pub connects_to_level: u8,

    /// Direction deg.
    pub direction_deg: f32,

    /// Step count.
    pub step_count: u16,

    /// Transparent steps.
    pub transparent_steps: bool,

    /// Los concealment.
    pub los_concealment: f32,
}

/// Furniture rec.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct FurnitureRec {
    /// Id.
    pub id: String,

    /// Name.
    pub name: String,

    /// Category.
    pub category: String,

    /// Position.
    pub position: [f32; 2],

    /// Rotation deg.
    pub rotation_deg: f32,

    /// Height m.
    pub height_m: f32,

    /// Blocks movement.
    pub blocks_movement: bool,

    /// Los cover.
    pub los_cover: String,
}

/// Building level.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct BuildingLevel {
    /// Level index.
    pub level_index: u8,

    /// Elevation range.
    pub elevation_range: [f32; 2],

    /// Footprint polygon.
    pub footprint_polygon: Vec<[f32; 2]>,

    /// Walls.
    pub walls: Vec<WallRec>,

    /// Doors.
    pub doors: Vec<DoorRec>,

    /// Windows.
    pub windows: Vec<WindowRec>,

    /// Stairs.
    pub stairs: Vec<StairsRec>,

    /// Furniture.
    pub furniture: Vec<FurnitureRec>,
}

/// Building blueprint.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct BuildingBlueprint {
    /// Prefab id.
    pub prefab_id: u32,

    /// Slug.
    pub slug: String,

    /// Vertical profile.
    pub vertical_profile: VerticalProfile,

    /// Levels.
    pub levels: Vec<BuildingLevel>,
}

/// Building blueprint archive.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct BuildingBlueprintArchive {
    /// Schema version.
    pub schema_version: u16,

    /// Descriptors.
    pub descriptors: Vec<OccluderDescriptor>,

    /// Blas index.
    pub blas_index: Vec<BlasEntry>,

    /// Blueprints.
    pub blueprints: Vec<BuildingBlueprint>,
}
