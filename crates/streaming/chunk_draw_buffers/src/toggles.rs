//! The world layer toggles, the atlas key order and the airfield box, and the lane visibility
//! they decide.
//!
//! **Role:** takes the page's trees, props, buildings, fences and airfield toggles, the glyph
//! atlas key order and the runway-derived airfield box, rebuilds the buffers each one changes, and
//! answers which strip and badge lanes may draw.
//! **Position:** `chunk_draw_buffers`; methods of
//! [`crate::world_residency::WorldResidency`] and visibility predicates of
//! `DrawBuffers` (`draw_buffers.rs`); called by the world loader's atlas,
//! bootstrap and viewport passes, the strip and glyph composers and the residency tests.
//! **Signals & state:** writes the draw buffers' toggles, icon key table and airfield box.
//! **Invariants:** a toggle set to its current value rebuilds nothing; the strip lane's
//! visibility depends on toggles and zoom only, never on buffer contents.

use crate::draw_buffers::DrawBuffers;
use crate::world_residency::WorldResidency;
use map_coordinates::chunk_math::Bbox;
use map_draw_lanes::zoom_gates::building_visible;
use map_draw_lanes::zoom_gates::class_visible;
use road_network::airfield::compute_airfield_bbox;

impl WorldResidency {
    /// Register atlas icon keys in UV-table order (must match `upload_glyph_atlas` UV order). Rebuilds the glyph prefab lookup when prefabs are already loaded.
    pub fn set_glyph_key_map(&mut self, keys: &[String]) {
        let residency = &self.chunk_residency;
        let buffers = &mut self.draw_buffers;
        buffers.icon_key_to_idx.clear();
        for (i, k) in keys.iter().enumerate() {
            if i < usize::from(u16::MAX) {
                buffers.icon_key_to_idx.insert(k.clone(), i as u16);
            }
        }
        buffers.rebuild_glyph_lookup_from_prefabs(residency);

        buffers.glyph_base_key = 0;
        buffers.strip_key = 0;
        buffers.refresh_draw_set_and_glyphs(residency);
    }
}

impl WorldResidency {
    /// User layer toggles (mirrors `worldLayerPrefs.classToggles` trees/props/buildings).
    pub fn set_glyph_toggles(&mut self, trees: bool, props: bool, buildings: bool) {
        let buffers = &mut self.draw_buffers;
        if buffers.toggle_trees == trees
            && buffers.toggle_props == props
            && buffers.toggle_buildings == buildings
        {
            return;
        }
        buffers.toggle_trees = trees;
        buffers.toggle_props = props;
        buffers.toggle_buildings = buildings;

        buffers.rebuild_buffers(&self.chunk_residency);
    }
}

impl WorldResidency {
    /// Set fences toggle.
    pub fn set_fences_toggle(&mut self, fences: bool) {
        if self.draw_buffers.toggle_fences == fences {
            return;
        }
        self.draw_buffers.toggle_fences = fences;
        self.draw_buffers
            .rebuild_strip_buffers(&self.chunk_residency);
    }
}

impl WorldResidency {
    /// Set airfield toggle.
    pub fn set_airfield_toggle(&mut self, on: bool) {
        if self.draw_buffers.toggle_airfield == on {
            return;
        }
        self.draw_buffers.toggle_airfield = on;
        self.draw_buffers
            .rebuild_glyph_buffers(&self.chunk_residency);
    }
}

impl WorldResidency {
    /// Set airfield bbox from runway segments (call after roads load).
    pub fn set_airfield_bbox_from_runways(
        &mut self,
        runways: &[road_network::network::RoadSegment],
    ) {
        self.draw_buffers.airfield_bbox = compute_airfield_bbox(runways);
        self.draw_buffers
            .rebuild_glyph_buffers(&self.chunk_residency);
    }
}

impl WorldResidency {
    /// Whether airfield-specific icons draw (user toggle ∧ bbox known).
    #[must_use]
    pub fn airfield_visible(&self) -> bool {
        self.draw_buffers.airfield_visible()
    }
}

impl WorldResidency {
    /// Airfield bbox.
    #[must_use]
    pub fn airfield_bbox(&self) -> Option<Bbox> {
        self.draw_buffers.airfield_bbox
    }
}

impl WorldResidency {
    /// Fences visible.
    #[must_use]
    pub fn fences_visible(&self) -> bool {
        self.draw_buffers
            .fences_visible(self.chunk_residency.deck_zoom())
    }
}

impl WorldResidency {
    /// Piers visible.
    #[must_use]
    pub fn piers_visible(&self) -> bool {
        self.draw_buffers
            .piers_visible(self.chunk_residency.deck_zoom())
    }
}

impl WorldResidency {
    /// Buildings visible.
    #[must_use]
    pub fn buildings_visible(&self) -> bool {
        self.draw_buffers
            .buildings_visible(self.chunk_residency.deck_zoom())
    }
}

impl WorldResidency {
    /// Whether the shared strip render lane (fences + piers + bridge rails) may draw at all — stable on toggle+zoom only (fences OR piers OR buildings). The loader passes this as the upload `visible` flag; keeping it independent of buffer contents preserves the empty+visible mid-hydration anti-wipe guard while the three sub-lanes gate independently.
    #[must_use]
    pub fn strips_visible(&self) -> bool {
        self.fences_visible() || self.piers_visible() || self.buildings_visible()
    }
}

impl DrawBuffers {
    /// Whether airfield-specific icons draw (user toggle ∧ bbox known).
    pub(super) fn airfield_visible(&self) -> bool {
        self.toggle_airfield && self.airfield_bbox.is_some()
    }

    /// Whether fences draw at `deck_zoom`.
    pub(super) fn fences_visible(&self, deck_zoom: f64) -> bool {
        self.toggle_fences && class_visible("fence", deck_zoom)
    }

    /// Whether piers and docks draw at `deck_zoom`.
    pub(super) fn piers_visible(&self, deck_zoom: f64) -> bool {
        self.toggle_buildings && class_visible("pier", deck_zoom)
    }

    /// Whether buildings draw at `deck_zoom`.
    pub(super) fn buildings_visible(&self, deck_zoom: f64) -> bool {
        self.toggle_buildings && building_visible(deck_zoom)
    }
}
