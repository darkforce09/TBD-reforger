//! The archive records as they are shaped with bare primitive identifier fields.
//!
//! **Role:** declares, for every archive record that holds an identifier, a twin whose
//! identifier fields are the bare `u32` or `String` the identifier types wrap, and the projection
//! of each archive onto its twin, so a test can compare the two serialisations byte for byte.
//! **Position:** test-only child of [`crate::archives`], used by
//! `archive_wire_identity_tests.rs`; records without an identifier field are reused as they are.
//! **Signals & state:** none; plain data types and pure functions.
//! **Invariants:** each twin lists the same fields, in the same order and of the same types, as
//! its record, except that every identifier field is the identifier's inner primitive.

use crate::archives::blueprints::{
    BlasEntry, BuildingBlueprint, BuildingBlueprintArchive, BuildingLevel, OccluderDescriptor,
    VerticalProfile,
};
use crate::archives::forest::ForestRegionsArchive;
use crate::archives::prefabs::{KindCensus, PrefabCatalogArchive, PrefabEntry};
use crate::archives::roads::RoadNetworkArchive;
use crate::archives::water::{WaterBody, WaterLine, WaterVectorsArchive};

/// Prefab entry with a bare `u32` prefab id.
#[derive(rkyv::Archive, rkyv::Serialize)]
pub(super) struct PrimitivePrefabEntry {
    prefab_id: u32,
    kind: String,
    class: String,
    class_code: u8,
    label: String,
    resource_name: String,
    half_extents: [f32; 3],
    height_m: f32,
    icon_key: String,
    base_size_px: f32,
    default_color: [u8; 4],
    importance_zoom: f32,
}

/// Type inventory with a bare `String` terrain id.
#[derive(rkyv::Archive, rkyv::Serialize)]
pub(super) struct PrimitiveTypeInventory {
    terrain_id: String,
    census_status: String,
    unique_prefabs: u32,
    total_instances: u64,
    by_kind: Vec<KindCensus>,
}

/// Prefab catalogue archive over the primitive twins.
#[derive(rkyv::Archive, rkyv::Serialize)]
pub(super) struct PrimitivePrefabCatalogArchive {
    schema_version: u16,
    prefabs: Vec<PrimitivePrefabEntry>,
    type_inventory: PrimitiveTypeInventory,
}

/// Road segment with a bare `String` id.
#[derive(rkyv::Archive, rkyv::Serialize)]
pub(super) struct PrimitiveRoadSegment {
    id: String,
    road_class: u8,
    width_m: f32,
    centerline: Vec<[f32; 2]>,
}

/// Road network archive over the primitive twin.
#[derive(rkyv::Archive, rkyv::Serialize)]
pub(super) struct PrimitiveRoadNetworkArchive {
    schema_version: u16,
    segments: Vec<PrimitiveRoadSegment>,
}

/// Forest region with a bare `String` id.
#[derive(rkyv::Archive, rkyv::Serialize)]
pub(super) struct PrimitiveForestRegion {
    id: String,
    kind: String,
    polygon: Vec<Vec<[f32; 2]>>,
    tree_count: u32,
    dominant_species_class: String,
    density_per_ha: f32,
    area_ha: f32,
    cover_type: String,
}

/// Forest regions archive over the primitive twin.
#[derive(rkyv::Archive, rkyv::Serialize)]
pub(super) struct PrimitiveForestRegionsArchive {
    schema_version: u16,
    regions: Vec<PrimitiveForestRegion>,
}

/// Water body with a bare `String` id.
#[derive(rkyv::Archive, rkyv::Serialize)]
pub(super) struct PrimitiveWaterBody {
    id: String,
    surface_y: f32,
    ring: Vec<[f32; 2]>,
}

/// Water line with a bare `String` id.
#[derive(rkyv::Archive, rkyv::Serialize)]
pub(super) struct PrimitiveWaterLine {
    id: String,
    width_m: f32,
    centerline: Vec<[f32; 2]>,
}

/// Water vectors archive over the primitive twins.
#[derive(rkyv::Archive, rkyv::Serialize)]
pub(super) struct PrimitiveWaterVectorsArchive {
    schema_version: u16,
    lakes: Vec<PrimitiveWaterBody>,
    rivers: Vec<PrimitiveWaterLine>,
    ponds: Vec<PrimitiveWaterBody>,
}

/// Occluder descriptor with a bare `u32` prefab id.
#[derive(rkyv::Archive, rkyv::Serialize)]
pub(super) struct PrimitiveOccluderDescriptor {
    prefab_id: u32,
    slug: String,
    kind: String,
    blocks: bool,
    canopy: bool,
    local_bounds: [[f32; 3]; 2],
    blas: Vec<u32>,
}

/// Wall with a bare `String` id.
#[derive(rkyv::Archive, rkyv::Serialize)]
pub(super) struct PrimitiveWall {
    id: String,
    start: [f32; 2],
    end: [f32; 2],
    thickness_m: f32,
    is_exterior: bool,
    material: String,
}

/// Door with bare `String` door and wall ids.
#[derive(rkyv::Archive, rkyv::Serialize)]
pub(super) struct PrimitiveDoor {
    id: String,
    wall_id: String,
    position: [f32; 2],
    width_m: f32,
    height_m: f32,
    is_exterior: bool,
    has_glass: bool,
}

/// Window with bare `String` window and wall ids.
#[derive(rkyv::Archive, rkyv::Serialize)]
pub(super) struct PrimitiveWindow {
    id: String,
    wall_id: String,
    position: [f32; 2],
    width_m: f32,
    sill_height_m: f32,
    window_height_m: f32,
    normal: [f32; 2],
    fov_deg: f32,
    has_glass: bool,
}

/// Staircase with a bare `String` id.
#[derive(rkyv::Archive, rkyv::Serialize)]
pub(super) struct PrimitiveStairs {
    id: String,
    bounds: [[f32; 2]; 2],
    connects_to_level: u8,
    direction_deg: f32,
    step_count: u16,
    transparent_steps: bool,
    los_concealment: f32,
}

/// Furniture with a bare `String` id.
#[derive(rkyv::Archive, rkyv::Serialize)]
pub(super) struct PrimitiveFurniture {
    id: String,
    name: String,
    category: String,
    position: [f32; 2],
    rotation_deg: f32,
    height_m: f32,
    blocks_movement: bool,
    los_cover: String,
}

/// Building level over the primitive element twins.
#[derive(rkyv::Archive, rkyv::Serialize)]
pub(super) struct PrimitiveBuildingLevel {
    level_index: u8,
    elevation_range: [f32; 2],
    footprint_polygon: Vec<[f32; 2]>,
    walls: Vec<PrimitiveWall>,
    doors: Vec<PrimitiveDoor>,
    windows: Vec<PrimitiveWindow>,
    stairs: Vec<PrimitiveStairs>,
    furniture: Vec<PrimitiveFurniture>,
}

/// Building blueprint with a bare `u32` prefab id.
#[derive(rkyv::Archive, rkyv::Serialize)]
pub(super) struct PrimitiveBuildingBlueprint {
    prefab_id: u32,
    slug: String,
    vertical_profile: VerticalProfile,
    levels: Vec<PrimitiveBuildingLevel>,
}

/// Building blueprint archive over the primitive twins.
#[derive(rkyv::Archive, rkyv::Serialize)]
pub(super) struct PrimitiveBuildingBlueprintArchive {
    schema_version: u16,
    descriptors: Vec<PrimitiveOccluderDescriptor>,
    blas_index: Vec<BlasEntry>,
    blueprints: Vec<PrimitiveBuildingBlueprint>,
}

fn prefab_entry(e: &PrefabEntry) -> PrimitivePrefabEntry {
    PrimitivePrefabEntry {
        prefab_id: e.prefab_id.get(),
        kind: e.kind.clone(),
        class: e.class.clone(),
        class_code: e.class_code,
        label: e.label.clone(),
        resource_name: e.resource_name.clone(),
        half_extents: e.half_extents,
        height_m: e.height_m,
        icon_key: e.icon_key.clone(),
        base_size_px: e.base_size_px,
        default_color: e.default_color,
        importance_zoom: e.importance_zoom,
    }
}

/// The prefab catalogue projected onto its primitive twin.
pub(super) fn prefab_catalog(a: &PrefabCatalogArchive) -> PrimitivePrefabCatalogArchive {
    let inventory = &a.type_inventory;
    PrimitivePrefabCatalogArchive {
        schema_version: a.schema_version,
        prefabs: a.prefabs.iter().map(prefab_entry).collect(),
        type_inventory: PrimitiveTypeInventory {
            terrain_id: inventory.terrain_id.as_str().to_owned(),
            census_status: inventory.census_status.clone(),
            unique_prefabs: inventory.unique_prefabs,
            total_instances: inventory.total_instances,
            by_kind: inventory.by_kind.clone(),
        },
    }
}

/// The road network projected onto its primitive twin.
pub(super) fn road_network(a: &RoadNetworkArchive) -> PrimitiveRoadNetworkArchive {
    PrimitiveRoadNetworkArchive {
        schema_version: a.schema_version,
        segments: a
            .segments
            .iter()
            .map(|s| PrimitiveRoadSegment {
                id: s.id.as_str().to_owned(),
                road_class: s.road_class,
                width_m: s.width_m,
                centerline: s.centerline.clone(),
            })
            .collect(),
    }
}

/// The forest regions projected onto their primitive twin.
pub(super) fn forest_regions(a: &ForestRegionsArchive) -> PrimitiveForestRegionsArchive {
    PrimitiveForestRegionsArchive {
        schema_version: a.schema_version,
        regions: a
            .regions
            .iter()
            .map(|r| PrimitiveForestRegion {
                id: r.id.as_str().to_owned(),
                kind: r.kind.clone(),
                polygon: r.polygon.clone(),
                tree_count: r.tree_count,
                dominant_species_class: r.dominant_species_class.clone(),
                density_per_ha: r.density_per_ha,
                area_ha: r.area_ha,
                cover_type: r.cover_type.clone(),
            })
            .collect(),
    }
}

fn water_body(b: &WaterBody) -> PrimitiveWaterBody {
    PrimitiveWaterBody {
        id: b.id.as_str().to_owned(),
        surface_y: b.surface_y,
        ring: b.ring.clone(),
    }
}

fn water_line(l: &WaterLine) -> PrimitiveWaterLine {
    PrimitiveWaterLine {
        id: l.id.as_str().to_owned(),
        width_m: l.width_m,
        centerline: l.centerline.clone(),
    }
}

/// The water vectors projected onto their primitive twin.
pub(super) fn water_vectors(a: &WaterVectorsArchive) -> PrimitiveWaterVectorsArchive {
    PrimitiveWaterVectorsArchive {
        schema_version: a.schema_version,
        lakes: a.lakes.iter().map(water_body).collect(),
        rivers: a.rivers.iter().map(water_line).collect(),
        ponds: a.ponds.iter().map(water_body).collect(),
    }
}

fn occluder(d: &OccluderDescriptor) -> PrimitiveOccluderDescriptor {
    PrimitiveOccluderDescriptor {
        prefab_id: d.prefab_id.get(),
        slug: d.slug.clone(),
        kind: d.kind.clone(),
        blocks: d.blocks,
        canopy: d.canopy,
        local_bounds: d.local_bounds,
        blas: d.blas.clone(),
    }
}

fn level(l: &BuildingLevel) -> PrimitiveBuildingLevel {
    PrimitiveBuildingLevel {
        level_index: l.level_index,
        elevation_range: l.elevation_range,
        footprint_polygon: l.footprint_polygon.clone(),
        walls: l
            .walls
            .iter()
            .map(|w| PrimitiveWall {
                id: w.id.as_str().to_owned(),
                start: w.start,
                end: w.end,
                thickness_m: w.thickness_m,
                is_exterior: w.is_exterior,
                material: w.material.clone(),
            })
            .collect(),
        doors: l
            .doors
            .iter()
            .map(|d| PrimitiveDoor {
                id: d.id.as_str().to_owned(),
                wall_id: d.wall_id.as_str().to_owned(),
                position: d.position,
                width_m: d.width_m,
                height_m: d.height_m,
                is_exterior: d.is_exterior,
                has_glass: d.has_glass,
            })
            .collect(),
        windows: l
            .windows
            .iter()
            .map(|w| PrimitiveWindow {
                id: w.id.as_str().to_owned(),
                wall_id: w.wall_id.as_str().to_owned(),
                position: w.position,
                width_m: w.width_m,
                sill_height_m: w.sill_height_m,
                window_height_m: w.window_height_m,
                normal: w.normal,
                fov_deg: w.fov_deg,
                has_glass: w.has_glass,
            })
            .collect(),
        stairs: l
            .stairs
            .iter()
            .map(|s| PrimitiveStairs {
                id: s.id.as_str().to_owned(),
                bounds: s.bounds,
                connects_to_level: s.connects_to_level,
                direction_deg: s.direction_deg,
                step_count: s.step_count,
                transparent_steps: s.transparent_steps,
                los_concealment: s.los_concealment,
            })
            .collect(),
        furniture: l
            .furniture
            .iter()
            .map(|f| PrimitiveFurniture {
                id: f.id.as_str().to_owned(),
                name: f.name.clone(),
                category: f.category.clone(),
                position: f.position,
                rotation_deg: f.rotation_deg,
                height_m: f.height_m,
                blocks_movement: f.blocks_movement,
                los_cover: f.los_cover.clone(),
            })
            .collect(),
    }
}

fn blueprint(b: &BuildingBlueprint) -> PrimitiveBuildingBlueprint {
    PrimitiveBuildingBlueprint {
        prefab_id: b.prefab_id.get(),
        slug: b.slug.clone(),
        vertical_profile: b.vertical_profile.clone(),
        levels: b.levels.iter().map(level).collect(),
    }
}

/// The building blueprint archive projected onto its primitive twin.
pub(super) fn building_blueprints(
    a: &BuildingBlueprintArchive,
) -> PrimitiveBuildingBlueprintArchive {
    PrimitiveBuildingBlueprintArchive {
        schema_version: a.schema_version,
        descriptors: a.descriptors.iter().map(occluder).collect(),
        blas_index: a.blas_index.clone(),
        blueprints: a.blueprints.iter().map(blueprint).collect(),
    }
}
