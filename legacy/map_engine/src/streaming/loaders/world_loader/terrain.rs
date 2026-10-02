//! Role: terrain.
//! Position: `streaming/loaders/world_loader` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::EngineHandle;
use super::LandcoverInput;
use super::RoadInput;
use super::WorldHost;
use super::compose_landcover_mesh;
use super::compose_roads_mesh;
use super::road_class_signature;

impl WorldHost {
    /// Push roads.
    pub(super) fn push_roads(&mut self, engine: &EngineHandle, zoom: f64) -> bool {
        if !self.roads_loaded {
            return false;
        }

        let sig = road_class_signature(zoom);
        if self.last_road_sig == Some(sig) {
            return false;
        }
        self.last_road_sig = Some(sig);
        if !self.road_meshes.contains_key(&sig) {
            let inputs: Vec<RoadInput<'_>> = self
                .store
                .roads
                .iter()
                .map(|r| RoadInput {
                    road_class: r.road_class.as_str(),
                    points: r.points.as_slice(),
                    width_m: r.width_m,
                })
                .collect();

            self.crossing_allocs.pass_staging += 2;
            self.road_meshes
                .insert(sig, compose_roads_mesh(&inputs, zoom, true));
        }
        let mesh = &self.road_meshes[&sig];
        let vis = mesh.segment_count > 0;
        if let Some(e) = engine.borrow_mut().as_mut() {
            e.upload_strip_tris(
                crate::overlay::lanes::role_id::ROADS_CASING,
                &mesh.casing,
                mesh.segment_count,
                vis,
            );
            e.upload_strip_tris(
                crate::overlay::lanes::role_id::ROADS,
                &mesh.centerline,
                mesh.segment_count,
                vis,
            );
        }
        true
    }
}

impl WorldHost {
    /// Push landcover.
    pub(super) fn push_landcover(&mut self, engine: &EngineHandle) -> bool {
        if !self.landcover_ready {
            return false;
        }
        let vis = self.residency.forest_fill_effective();

        if vis == self.landcover_shown && (self.landcover_mesh.is_some() || !vis) {
            return false;
        }
        if !vis {
            self.landcover_shown = false;
            if let Some(e) = engine.borrow_mut().as_mut() {
                e.clear_vector_lane(crate::overlay::lanes::role_id::LANDCOVER);
            }
            return true;
        }
        if self.landcover_mesh.is_none() {
            let inputs: Vec<LandcoverInput<'_>> = self
                .store
                .regions
                .iter()
                .filter(|r| r.kind.as_str() != "forest")
                .map(|r| LandcoverInput {
                    kind: r.kind.as_str(),
                    rings: r.polygon.as_slice(),
                })
                .collect();

            self.crossing_allocs.pass_staging += 2;
            self.landcover_mesh = Some(compose_landcover_mesh(&inputs));
        }
        self.landcover_shown = true;
        if let Some(mesh) = &self.landcover_mesh
            && let Some(e) = engine.borrow_mut().as_mut()
        {
            if mesh.polygon_count > 0 {
                e.upload_polygon_mesh(
                    crate::overlay::lanes::role_id::LANDCOVER,
                    &mesh.positions,
                    &mesh.colors,
                    &mesh.indices,
                    mesh.polygon_count,
                    true,
                );
            } else {
                e.clear_vector_lane(crate::overlay::lanes::role_id::LANDCOVER);
            }
        }
        true
    }
}
