//! Role: viewport.
//! Position: `streaming/loaders/world_loader` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::BootEvent;
use super::BridgeHandle;
use super::EngineHandle;

use super::WorldHost;

/// Canonical fetch concurrency value.
pub(super) const FETCH_CONCURRENCY: usize = 12;

impl WorldHost {
    /// Run viewport.
    pub async fn run_viewport(
        &mut self,
        engine: &EngineHandle,
        bridge: &BridgeHandle,
        report: &dyn Fn(BootEvent),
    ) -> bool {
        if !self.ready {
            return false;
        }
        self.crossing_allocs.reset_pass();
        self.ensure_atlas(engine, bridge);
        let (bounds, zoom) = {
            let g = engine.borrow();
            let Some(e) = g.as_ref() else {
                return false;
            };
            (e.visible_bounds(), e.zoom())
        };
        if bounds.len() < 4 {
            return false;
        }

        self.apply_layer_prefs(engine);
        let roads_changed = self.push_roads(engine, zoom);
        let had_resident = self.residency.chunks_resident() > 0;
        let missing = self
            .residency
            .set_viewport(bounds[0], bounds[1], bounds[2], bounds[3], zoom);

        let landcover_changed = self.push_landcover(engine);

        let fetched = !missing.is_empty();
        if fetched {
            self.fetch_and_queue(missing, report).await;
        }
        let drained = self.drain(engine, bridge);
        let pushed = self.push_to_engine(engine, bridge);
        if fetched && had_resident {
            self.crossing_allocs.commit_warm();
        }

        let occluded = self.occluder.run_viewport(&mut self.residency).await;
        roads_changed || landcover_changed || fetched || drained || pushed || occluded
    }
}

impl WorldHost {
    /// Apply layer prefs.
    pub(super) fn apply_layer_prefs(&mut self, engine: &EngineHandle) {
        let p = (self.world_layers)();

        self.residency
            .set_glyph_toggles(p.trees, p.props, p.buildings);
        self.residency.set_fences_toggle(p.fences);
        self.residency.set_airfield_toggle(p.airfield);

        if let Some(e) = engine.borrow_mut().as_mut() {
            e.set_world_layer_visible("roads", p.roads);
            e.set_world_layer_visible("forest", p.forest);
            e.set_world_layer_visible("contours", p.contours);
            e.set_world_layer_visible("sea", p.sea);
            e.set_world_layer_visible("airfield", p.airfield);
            e.set_world_layer_visible("heights", p.heights);
            e.set_world_layer_visible("townLabels", p.town_labels);
            e.set_world_layer_visible("roadNames", p.road_names);
        }
    }
}

impl WorldHost {
    /// Occluder.
    #[must_use]
    pub fn occluder(&self) -> &crate::spatial::los::world::state::WorldOccluder {
        self.occluder.occluder()
    }
}

impl WorldHost {
    /// The occluder host itself (fetch bookkeeping: the failed set).
    #[must_use]
    pub fn occluder_host(&self) -> &crate::streaming::loaders::occluder_loader::OccluderHost {
        &self.occluder
    }
}

impl WorldHost {
    /// Road segments clone.
    #[must_use]
    pub fn road_segments_clone(&self) -> Vec<crate::world::terrain::roads::network::RoadSegment> {
        self.store.roads.clone()
    }
}
