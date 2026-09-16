//! Role: statistics.
//! Position: `streaming/bridge` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use wasm_bindgen::JsValue;

/// Map assets bridge.
#[derive(Default, Clone)]
pub struct MapAssetsBridge {
    /// Hillshade w.
    pub hillshade_w: u32,

    /// Hillshade h.
    pub hillshade_h: u32,

    /// Sat w.
    pub sat_w: u32,

    /// Sat h.
    pub sat_h: u32,

    /// Sat mode.
    pub sat_mode: String,

    /// Sat mips.
    pub sat_mips: u32,

    /// Glyph atlas.
    pub glyph_atlas: bool,

    /// Basemap mode.
    pub basemap_mode: String,

    /// Road segments.
    pub road_segments: u32,

    /// Landcover polygons.
    pub landcover_polygons: u32,

    /// Sea polygons.
    pub sea_polygons: u32,

    /// Contour segments.
    pub contour_segments: u32,

    /// Forest polygons.
    pub forest_polygons: u32,

    /// Forest outline segments.
    pub forest_outline_segments: u32,

    /// Forest density w.
    pub forest_density_w: u32,

    /// Forest density h.
    pub forest_density_h: u32,

    /// Forest bins ok.
    pub forest_bins_ok: u32,

    /// Forest mode.
    pub forest_mode: String,

    /// World building instances.
    pub world_building_instances: u32,

    /// World chunks drawn.
    pub world_chunks_drawn: u32,

    /// Tree glyphs.
    pub tree_glyphs: u32,

    /// Atlas bytes.
    pub atlas_bytes: u64,

    /// Residency-side packed count (CDP / verify-log).
    pub tree_glyph_packed: u32,

    /// Icon lane uploads.
    pub icon_lane_uploads: u64,

    /// Polygon lane uploads.
    pub polygon_lane_uploads: u64,

    /// Strip lane uploads.
    pub strip_lane_uploads: u64,

    /// Building uploads.
    pub building_uploads: u64,

    /// Text label uploads.
    pub text_label_uploads: u64,

    /// Buffers revision.
    pub buffers_revision: u64,

    /// Glyph recomposes.
    pub glyph_recomposes: u64,

    /// Fill recomposes.
    pub fill_recomposes: u64,
}

impl MapAssetsBridge {
    /// Install.
    pub fn install(&self) {
        let Some(win) = web_sys::window() else {
            return;
        };
        let obj = js_sys::Object::new();
        let set = |k: &str, v: JsValue| {
            let _ = js_sys::Reflect::set(&obj, &JsValue::from_str(k), &v);
        };
        set("hillshadeW", JsValue::from_f64(f64::from(self.hillshade_w)));
        set("hillshadeH", JsValue::from_f64(f64::from(self.hillshade_h)));
        set("satW", JsValue::from_f64(f64::from(self.sat_w)));
        set("satH", JsValue::from_f64(f64::from(self.sat_h)));
        set("satMode", JsValue::from_str(&self.sat_mode));
        set("satMips", JsValue::from_f64(f64::from(self.sat_mips)));
        set("glyphAtlas", JsValue::from_bool(self.glyph_atlas));
        set("basemap_mode", JsValue::from_str(&self.basemap_mode));
        set(
            "road_segments",
            JsValue::from_f64(f64::from(self.road_segments)),
        );
        set(
            "landcover_polygons",
            JsValue::from_f64(f64::from(self.landcover_polygons)),
        );
        set(
            "sea_polygons",
            JsValue::from_f64(f64::from(self.sea_polygons)),
        );
        set(
            "contour_segments",
            JsValue::from_f64(f64::from(self.contour_segments)),
        );
        set(
            "forest_polygons",
            JsValue::from_f64(f64::from(self.forest_polygons)),
        );
        set(
            "forest_outline_segments",
            JsValue::from_f64(f64::from(self.forest_outline_segments)),
        );
        set(
            "forest_density_w",
            JsValue::from_f64(f64::from(self.forest_density_w)),
        );
        set(
            "forest_density_h",
            JsValue::from_f64(f64::from(self.forest_density_h)),
        );
        set(
            "forest_bins_ok",
            JsValue::from_f64(f64::from(self.forest_bins_ok)),
        );
        set("forest_mode", JsValue::from_str(&self.forest_mode));
        set(
            "world_building_instances",
            JsValue::from_f64(f64::from(self.world_building_instances)),
        );
        set(
            "world_chunks_drawn",
            JsValue::from_f64(f64::from(self.world_chunks_drawn)),
        );
        set(
            "tree_glyphs",
            JsValue::from_f64(f64::from(self.tree_glyphs)),
        );
        set("atlas_bytes", JsValue::from_f64(self.atlas_bytes as f64));
        set(
            "tree_glyph_packed",
            JsValue::from_f64(f64::from(self.tree_glyph_packed)),
        );
        set(
            "icon_lane_uploads",
            JsValue::from_f64(self.icon_lane_uploads as f64),
        );
        set(
            "polygon_lane_uploads",
            JsValue::from_f64(self.polygon_lane_uploads as f64),
        );
        set(
            "strip_lane_uploads",
            JsValue::from_f64(self.strip_lane_uploads as f64),
        );
        set(
            "building_uploads",
            JsValue::from_f64(self.building_uploads as f64),
        );
        set(
            "text_label_uploads",
            JsValue::from_f64(self.text_label_uploads as f64),
        );
        set(
            "buffers_revision",
            JsValue::from_f64(self.buffers_revision as f64),
        );
        set(
            "glyph_recomposes",
            JsValue::from_f64(self.glyph_recomposes as f64),
        );
        set(
            "fill_recomposes",
            JsValue::from_f64(self.fill_recomposes as f64),
        );
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__mapAssets"), &obj);
    }

    /// Merge selected fields from `engine.stats()` JSON into this bridge.
    pub fn merge_engine_stats(&mut self, stats_json: &str) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(stats_json) else {
            return;
        };
        let u32f = |k: &str| v.get(k).and_then(|x| x.as_u64()).map(|x| x as u32);
        if let Some(m) = v.get("basemap_mode").and_then(|x| x.as_str()) {
            self.basemap_mode = m.to_string();
        }
        if let Some(n) = u32f("road_segments") {
            self.road_segments = n;
        }
        if let Some(n) = u32f("landcover_polygons") {
            self.landcover_polygons = n;
        }
        if let Some(n) = u32f("sea_polygons") {
            self.sea_polygons = n;
        }
        if let Some(n) = u32f("contour_segments") {
            self.contour_segments = n;
        }
        if let Some(n) = u32f("forest_polygons") {
            self.forest_polygons = n;
        }
        if let Some(n) = u32f("forest_outline_segments") {
            self.forest_outline_segments = n;
        }
        if let Some(n) = u32f("forest_density_w") {
            self.forest_density_w = n;
        }
        if let Some(n) = u32f("forest_density_h") {
            self.forest_density_h = n;
        }
        if let Some(n) = u32f("forest_bins_ok") {
            self.forest_bins_ok = n;
        }
        if let Some(m) = v.get("forest_mode").and_then(|x| x.as_str()) {
            self.forest_mode = m.to_string();
        }
        if let Some(n) = u32f("world_building_instances") {
            self.world_building_instances = n;
        }
        if let Some(n) = u32f("world_chunks_drawn") {
            self.world_chunks_drawn = n;
        }
        if let Some(n) = u32f("tree_glyphs") {
            self.tree_glyphs = n;
        }
        if let Some(n) = v.get("atlas_bytes").and_then(|x| x.as_u64()) {
            self.atlas_bytes = n;
        }
        let u64f = |k: &str| v.get(k).and_then(serde_json::Value::as_u64);
        if let Some(n) = u64f("icon_lane_uploads") {
            self.icon_lane_uploads = n;
        }
        if let Some(n) = u64f("polygon_lane_uploads") {
            self.polygon_lane_uploads = n;
        }
        if let Some(n) = u64f("strip_lane_uploads") {
            self.strip_lane_uploads = n;
        }
        if let Some(n) = u64f("building_uploads") {
            self.building_uploads = n;
        }
        if let Some(n) = u64f("text_label_uploads") {
            self.text_label_uploads = n;
        }
    }

    /// Merge residency stats.
    pub fn merge_residency_stats(&mut self, stats_json: &str) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(stats_json) else {
            return;
        };
        let u64f = |k: &str| v.get(k).and_then(serde_json::Value::as_u64);
        if let Some(n) = u64f("buffers_revision") {
            self.buffers_revision = n;
        }
        if let Some(n) = u64f("glyph_recomposes") {
            self.glyph_recomposes = n;
        }
        if let Some(n) = u64f("fill_recomposes") {
            self.fill_recomposes = n;
        }
    }
}

/// Bridge handle.
pub type BridgeHandle = std::rc::Rc<std::cell::RefCell<MapAssetsBridge>>;

/// New bridge.
pub fn new_bridge() -> BridgeHandle {
    std::rc::Rc::new(std::cell::RefCell::new(MapAssetsBridge::default()))
}

/// Publish.
pub fn publish(bridge: &BridgeHandle) {
    bridge.borrow().install();
}

/// Publish engine.
pub fn publish_engine(bridge: &BridgeHandle, engine: &crate::frame::engine::RenderEngine) {
    {
        let mut b = bridge.borrow_mut();
        b.merge_engine_stats(&engine.stats());
    }
    publish(bridge);
}
