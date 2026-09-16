//! Role: archive.
//! Position: `architecture/blueprint` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::architecture::blueprint::footprint::OverallFootprint;
use crate::architecture::blueprint::footprint::VerticalProfile;
use crate::architecture::blueprint::structure::BBox2D;
use crate::architecture::blueprint::structure::BuildingBlueprint;
use crate::architecture::blueprint::structure::BuildingDoor;
use crate::architecture::blueprint::structure::BuildingFurniture;
use crate::architecture::blueprint::structure::BuildingLevel;
use crate::architecture::blueprint::structure::BuildingStairs;
use crate::architecture::blueprint::structure::BuildingWall;
use crate::architecture::blueprint::structure::BuildingWindow;

#[cfg(feature = "io")]
impl BuildingBlueprint {
    /// Rebuild the tactical subset from an archived blueprint.
    #[must_use]
    pub fn from_archived(
        a: &crate::formats::archives::blueprints::ArchivedBuildingBlueprint,
    ) -> Self {
        Self {
            schema_version: String::new(),
            prefab_id: a.slug.to_string(),
            resource_name: String::new(),
            model_mesh: None,
            label: None,
            kind: String::new(),
            category: String::new(),
            destructible: false,
            vertical_profile: VerticalProfile {
                pivot_elevation_offset_m: f64::from(
                    a.vertical_profile.pivot_elevation_offset_m.to_native(),
                ),
                foundation_skirt_depth_m: f64::from(
                    a.vertical_profile.foundation_skirt_depth_m.to_native(),
                ),
                total_height_m: f64::from(a.vertical_profile.total_height_m.to_native()),
                eave_height_m: f64::from(a.vertical_profile.eave_height_m.to_native()),
                ridge_height_m: f64::from(a.vertical_profile.ridge_height_m.to_native()),
                chimney_height_m: None,
                roof_type: a.vertical_profile.roof_type.to_string(),
            },
            overall_footprint: OverallFootprint {
                polygon2_d: Vec::new(),
                bounding_box2_d: BBox2D {
                    min: [0.0; 2],
                    max: [0.0; 2],
                    width_m: 0.0,
                    depth_m: 0.0,
                },
                footprint_sq_m: 0.0,
            },
            roof: None,
            levels: a.levels.iter().map(level_from_archived).collect(),
        }
    }
}

/// Pair.
#[cfg(feature = "io")]
pub(crate) fn pair(p: &[rkyv::rend::f32_le; 2]) -> [f64; 2] {
    [f64::from(p[0].to_native()), f64::from(p[1].to_native())]
}

/// Level from archived.
#[cfg(feature = "io")]
pub(crate) fn level_from_archived(
    a: &crate::formats::archives::blueprints::ArchivedBuildingLevel,
) -> BuildingLevel {
    BuildingLevel {
        level_index: usize::from(a.level_index),
        name: String::new(),
        elevation_range: pair(&a.elevation_range),
        slice_height_m: 0.0,
        footprint_polygon: a.footprint_polygon.iter().map(pair).collect(),
        plate: None,
        floor_polygons: Vec::new(),
        walls: a
            .walls
            .iter()
            .map(|w| BuildingWall {
                id: w.id.to_string(),
                start: pair(&w.start),
                end: pair(&w.end),
                thickness: f64::from(w.thickness_m.to_native()),
                is_exterior: w.is_exterior,
                material: w.material.to_string(),
            })
            .collect(),
        doors: a
            .doors
            .iter()
            .map(|d| BuildingDoor {
                id: d.id.to_string(),
                prefab_resource: String::new(),
                wall_id: d.wall_id.to_string(),
                pos2_d: pair(&d.position),
                width_m: f64::from(d.width_m.to_native()),
                height_m: f64::from(d.height_m.to_native()),
                hinge_side: String::new(),
                swing_direction: String::new(),
                is_exterior: d.is_exterior,
                has_glass: d.has_glass,
                default_state: String::new(),
            })
            .collect(),
        windows: a
            .windows
            .iter()
            .map(|w| BuildingWindow {
                id: w.id.to_string(),
                prefab_resource: String::new(),
                wall_id: w.wall_id.to_string(),
                pos2_d: pair(&w.position),
                width_m: f64::from(w.width_m.to_native()),
                sill_height_m: f64::from(w.sill_height_m.to_native()),
                window_height_m: f64::from(w.window_height_m.to_native()),
                normal: pair(&w.normal),
                fov_deg: f64::from(w.fov_deg.to_native()),
                has_glass: w.has_glass,
                glass_pane_count: 0,
            })
            .collect(),
        stairs: a
            .stairs
            .iter()
            .map(|s| BuildingStairs {
                id: s.id.to_string(),
                bounds: [pair(&s.bounds[0]), pair(&s.bounds[1])],
                connects_to_level: usize::from(s.connects_to_level),
                direction_deg: f64::from(s.direction_deg.to_native()),
                step_count: u32::from(s.step_count.to_native()),
                transparent_steps: s.transparent_steps,
                los_concealment: f64::from(s.los_concealment.to_native()),
            })
            .collect(),
        furniture: a
            .furniture
            .iter()
            .map(|f| BuildingFurniture {
                id: f.id.to_string(),
                name: f.name.to_string(),
                category: f.category.to_string(),
                prefab_resource: String::new(),
                pos2_d: pair(&f.position),
                rotation_deg: f64::from(f.rotation_deg.to_native()),
                size2_d: [0.0; 2],
                height_m: f64::from(f.height_m.to_native()),
                blocks_movement: f.blocks_movement,
                los_cover: f.los_cover.to_string(),
            })
            .collect(),
    }
}
