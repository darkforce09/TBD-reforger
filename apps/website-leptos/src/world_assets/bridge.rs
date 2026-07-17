//! `window.__mapAssets` — CDP bridge (T-159.28 / T-166).
//! Hillshade camelCase fields kept for `smoke_hillshade`; engine.stats snake_case keys are also
//! published so `smoke_fullmap` can assert Class-R pins without inventing aliases.

use wasm_bindgen::JsValue;

#[derive(Default, Clone)]
pub struct MapAssetsBridge {
    pub hillshade_w: u32,
    pub hillshade_h: u32,
    pub sat_w: u32,
    pub sat_h: u32,
    pub sat_mode: String,
    pub sat_mips: u32,
    pub glyph_atlas: bool,
    // engine.stats mirrors (snake_case — exact keys from map-engine-render)
    pub basemap_mode: String,
    pub road_segments: u32,
    pub landcover_polygons: u32,
    pub sea_polygons: u32,
    pub contour_segments: u32,
    pub forest_polygons: u32,
    pub forest_outline_segments: u32,
    pub world_building_instances: u32,
    pub world_chunks_drawn: u32,
    pub tree_glyphs: u32,
    pub atlas_bytes: u64,
    /// Residency-side packed count (CDP / verify-log).
    pub tree_glyph_packed: u32,
    pub heatmap_trees: bool,
}

impl MapAssetsBridge {
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
        set("heatmap_trees", JsValue::from_bool(self.heatmap_trees));
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
    }
}

pub type BridgeHandle = std::rc::Rc<std::cell::RefCell<MapAssetsBridge>>;

pub fn new_bridge() -> BridgeHandle {
    std::rc::Rc::new(std::cell::RefCell::new(MapAssetsBridge::default()))
}

pub fn publish(bridge: &BridgeHandle) {
    bridge.borrow().install();
}

pub fn publish_engine(bridge: &BridgeHandle, engine: &map_engine_render::RenderEngine) {
    {
        let mut b = bridge.borrow_mut();
        b.merge_engine_stats(&engine.stats());
    }
    publish(bridge);
}
