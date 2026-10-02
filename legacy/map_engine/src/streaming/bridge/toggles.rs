//! Role: toggles.
//! Position: `streaming/bridge` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::overlay::lod::class_visible;
use crate::streaming::scheduler::chunk_math::Bbox;
use crate::streaming::scheduler::state::WorldResidency;
use crate::world::environment::buildings::footprint::building_visible;
use crate::world::terrain::roads::airfield::compute_airfield_bbox;

impl WorldResidency {
    /// Register atlas icon keys in UV-table order (must match `upload_glyph_atlas` UV order). Rebuilds the glyph prefab lookup when prefabs are already loaded.
    pub fn set_glyph_key_map(&mut self, keys: &[String]) {
        self.icon_key_to_idx.clear();
        for (i, k) in keys.iter().enumerate() {
            if i < usize::from(u16::MAX) {
                self.icon_key_to_idx.insert(k.clone(), i as u16);
            }
        }
        self.rebuild_glyph_lookup_from_prefabs();

        self.glyph_base_key = 0;
        self.strip_key = 0;
        self.refresh_draw_set_and_glyphs();
    }
}

impl WorldResidency {
    /// User layer toggles (mirrors `worldLayerPrefs.classToggles` trees/props/buildings).
    pub fn set_glyph_toggles(&mut self, trees: bool, props: bool, buildings: bool) {
        if self.toggle_trees == trees
            && self.toggle_props == props
            && self.toggle_buildings == buildings
        {
            return;
        }
        self.toggle_trees = trees;
        self.toggle_props = props;
        self.toggle_buildings = buildings;

        self.rebuild_buffers();
    }
}

impl WorldResidency {
    /// Set fences toggle.
    pub fn set_fences_toggle(&mut self, fences: bool) {
        if self.toggle_fences == fences {
            return;
        }
        self.toggle_fences = fences;
        self.rebuild_strip_buffers();
    }
}

impl WorldResidency {
    /// Set airfield toggle.
    pub fn set_airfield_toggle(&mut self, on: bool) {
        if self.toggle_airfield == on {
            return;
        }
        self.toggle_airfield = on;
        self.rebuild_glyph_buffers();
    }
}

impl WorldResidency {
    /// Set airfield bbox from runway segments (call after roads load).
    pub fn set_airfield_bbox_from_runways(
        &mut self,
        runways: &[crate::world::terrain::roads::network::RoadSegment],
    ) {
        self.airfield_bbox = compute_airfield_bbox(runways);
        self.rebuild_glyph_buffers();
    }
}

impl WorldResidency {
    /// Whether airfield-specific icons draw (user toggle ∧ bbox known).
    #[must_use]
    pub fn airfield_visible(&self) -> bool {
        self.toggle_airfield && self.airfield_bbox.is_some()
    }
}

impl WorldResidency {
    /// Airfield bbox.
    #[must_use]
    pub fn airfield_bbox(&self) -> Option<Bbox> {
        self.airfield_bbox
    }
}

impl WorldResidency {
    /// Fences visible.
    #[must_use]
    pub fn fences_visible(&self) -> bool {
        self.toggle_fences && class_visible("fence", self.deck_zoom)
    }
}

impl WorldResidency {
    /// Piers visible.
    #[must_use]
    pub fn piers_visible(&self) -> bool {
        self.toggle_buildings && class_visible("pier", self.deck_zoom)
    }
}

impl WorldResidency {
    /// Buildings visible.
    #[must_use]
    pub fn buildings_visible(&self) -> bool {
        self.toggle_buildings && building_visible(self.deck_zoom)
    }
}

impl WorldResidency {
    /// Whether the shared strip render lane (fences + piers + bridge rails) may draw at all — stable on toggle+zoom only (fences OR piers OR buildings). The loader passes this as the upload `visible` flag; keeping it independent of buffer contents preserves the empty+visible mid-hydration anti-wipe guard while the three sub-lanes gate independently.
    #[must_use]
    pub fn strips_visible(&self) -> bool {
        self.fences_visible() || self.piers_visible() || self.buildings_visible()
    }
}
