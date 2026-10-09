//! The draw state composed from the chunk residency: the glyph lookup, the layer toggles, the
//! draw set, the packed buffers and their memo keys.
//!
//! **Role:** holds everything the world loader uploads (building fill and outline, strips, tree,
//! prop and badge glyphs, lane flags) and the state that decides when to recompose it.
//! **Position:** `chunk_draw_buffers`; one field of the composed
//! `crate::world_residency::WorldResidency`; the other buffers files add its
//! compose methods, each reading a `chunk_scheduler::state::ChunkResidency`.
//! **Signals & state:** every field is private to the buffers folder and written only by the
//! compose methods and the toggles.
//! **Invariants:** `buffers_revision` grows whenever a refresh recomposed a glyph or strip buffer;
//! a memo key equal to the stored one recomposes nothing.

use map_coordinates::chunk_math::Bbox;
use std::collections::HashMap;
use world_chunks::ChunkId;

/// One prefab's glyph: atlas index, size in metres, tint and compose group.
#[derive(Clone, Debug)]
pub(super) struct GlyphPrefabInfo {
    /// Glyph idx.
    pub(super) glyph_idx: u16,

    /// Size m.
    pub(super) size_m: f32,

    /// Tint.
    pub(super) tint: u32,

    /// Group: 0 trees and vegetation, 1 props and large rocks, 2 building badges.
    pub(super) group: u8,
}

/// The draw state composed over the resident chunks.
pub(crate) struct DrawBuffers {
    /// Glyph by u16.
    pub(super) glyph_by_u16: HashMap<u16, GlyphPrefabInfo>,

    /// Icon key to idx.
    pub(super) icon_key_to_idx: HashMap<String, u16>,

    /// Glyph base key.
    pub(super) glyph_base_key: u64,

    /// Strip key.
    pub(super) strip_key: u64,

    /// Buffers revision.
    pub(super) buffers_revision: u64,

    /// Glyph recomposes.
    pub(super) glyph_recomposes: u64,

    /// Fill recomposes.
    pub(super) fill_recomposes: u64,

    /// Fill buf.
    pub(super) fill_buf: Vec<f32>,

    /// Outline buf.
    pub(super) outline_buf: Vec<f32>,

    /// Strip buf.
    pub(super) strip_buf: Vec<f32>,

    /// Fence strip count.
    pub(super) fence_strip_count: u32,

    /// Pier strip count.
    pub(super) pier_strip_count: u32,

    /// Bridge rail count.
    pub(super) bridge_rail_count: u32,

    /// Toggle trees.
    pub(super) toggle_trees: bool,

    /// Toggle props.
    pub(super) toggle_props: bool,

    /// Toggle buildings.
    pub(super) toggle_buildings: bool,

    /// Toggle fences.
    pub(super) toggle_fences: bool,

    /// Toggle airfield.
    pub(super) toggle_airfield: bool,

    /// Airfield bbox.
    pub(super) airfield_bbox: Option<Bbox>,

    /// Tree glyph buf.
    pub(super) tree_glyph_buf: Vec<u8>,

    /// Prop glyph buf.
    pub(super) prop_glyph_buf: Vec<u8>,

    /// Badge glyph buf.
    pub(super) badge_glyph_buf: Vec<u8>,

    /// Tree want.
    pub(super) tree_want: bool,

    /// Prop want.
    pub(super) prop_want: bool,

    /// Badge want.
    pub(super) badge_want: bool,

    /// Draw ids.
    pub(super) draw_ids: Vec<ChunkId>,

    /// Heatmap trees.
    pub(super) heatmap_trees: bool,

    /// Exact tree count.
    pub(super) exact_tree_count: u32,

    /// Glyph size floor zoom.
    pub(super) glyph_size_floor_zoom: f64,
}

impl Default for DrawBuffers {
    fn default() -> Self {
        Self {
            glyph_by_u16: HashMap::new(),
            icon_key_to_idx: HashMap::new(),
            glyph_base_key: 0,
            strip_key: 0,
            buffers_revision: 0,
            glyph_recomposes: 0,
            fill_recomposes: 0,
            fill_buf: Vec::new(),
            outline_buf: Vec::new(),
            strip_buf: Vec::new(),
            fence_strip_count: 0,
            pier_strip_count: 0,
            bridge_rail_count: 0,
            toggle_trees: true,
            toggle_props: false,
            toggle_buildings: true,
            toggle_fences: true,
            toggle_airfield: true,
            airfield_bbox: None,
            tree_glyph_buf: Vec::new(),
            prop_glyph_buf: Vec::new(),
            badge_glyph_buf: Vec::new(),
            tree_want: false,
            prop_want: false,
            badge_want: false,
            draw_ids: Vec::new(),
            heatmap_trees: false,
            exact_tree_count: 0,
            glyph_size_floor_zoom: 0.0,
        }
    }
}
