//! Tests of the shared sink handle: an empty slot hands out nothing, a filled one hands out its
//! sink, writes through the handle reach the sink in call order, and a renderer cell coerces to
//! the handle.

use super::*;
use crate::{Error, Result};
use render_primitives::draw::compose::{HairlineGpu, PolyMeshGpu};
use std::cell::RefCell;
use std::rc::Rc;

/// A sink that records every write as one line, in call order.
#[derive(Default)]
struct RecordingSink {
    calls: Vec<String>,
    refuse_textures: bool,
}

impl MapViewport for RecordingSink {
    fn zoom(&self) -> f64 {
        -2.0
    }

    fn target_x(&self) -> f64 {
        6_400.0
    }

    fn target_y(&self) -> f64 {
        3_200.0
    }

    fn visible_bounds(&self) -> Option<[f64; 4]> {
        Some([0.0, 0.0, 12_800.0, 12_800.0])
    }

    fn set_view(&mut self, target_x: f64, target_y: f64, zoom: f64) {
        self.calls
            .push(format!("set_view {target_x} {target_y} {zoom}"));
    }

    fn on_camera_changed(&mut self) {
        self.calls.push("on_camera_changed".into());
    }
}

impl MapAssetSink for RecordingSink {
    type BrowserImage = u32;

    fn backend_is_webgl2(&self) -> bool {
        false
    }

    fn max_texture_dimension_2d(&self) -> u32 {
        8_192
    }

    fn adapter_max_texture_dimension_2d(&self) -> u32 {
        16_384
    }

    fn stats_json(&self) -> String {
        format!("{{\"calls\":{}}}", self.calls.len())
    }

    fn set_world_layer_visible(&mut self, layer: &str, visible: bool) {
        self.calls.push(format!("layer {layer} {visible}"));
    }

    fn set_lane_opacity(&mut self, role: u32, opacity: f32, visible: bool) {
        self.calls
            .push(format!("opacity {role} {opacity} {visible}"));
    }

    fn set_grid(&mut self, width_m: f64, height_m: f64, over_hillshade: bool, visible: bool) {
        self.calls.push(format!(
            "grid {width_m} {height_m} {over_hillshade} {visible}"
        ));
    }

    fn clear_vector_lane(&mut self, role: u32) {
        self.calls.push(format!("clear {role}"));
    }

    fn upload_polygon_mesh(&mut self, role: u32, mesh: &PolyMeshGpu, visible: bool) {
        self.calls
            .push(format!("polygons {role} {} {visible}", mesh.polygon_count));
    }

    fn upload_strip_tris(&mut self, role: u32, packed: &[f32], item_count: u32, visible: bool) {
        self.calls.push(format!(
            "strips {role} {} {item_count} {visible}",
            packed.len()
        ));
    }

    fn upload_hairline_segments(&mut self, role: u32, hairlines: &HairlineGpu, visible: bool) {
        self.calls.push(format!(
            "hairlines {role} {} {visible}",
            hairlines.segment_count
        ));
    }

    fn upload_world_buildings(&mut self, fill: &[f32], chunk_count: u32, visible: bool) {
        self.calls
            .push(format!("buildings {} {chunk_count} {visible}", fill.len()));
    }

    fn upload_world_building_outlines(&mut self, lines: &[f32], visible: bool) {
        self.calls
            .push(format!("outlines {} {visible}", lines.len()));
    }

    fn upload_world_fence_strips(&mut self, packed: &[f32], item_count: u32, visible: bool) {
        self.calls
            .push(format!("fences {} {item_count} {visible}", packed.len()));
    }

    fn upload_icon_lane(&mut self, lane: WorldGlyphLane, bytes: &[u8], visible: bool) {
        self.calls
            .push(format!("icons {lane:?} {} {visible}", bytes.len()));
    }

    fn upload_glyph_atlas(&mut self, atlas: &GlyphAtlasImage<'_>) -> Result<()> {
        self.calls.push(format!(
            "atlas {}x{} {}",
            atlas.width,
            atlas.height,
            atlas.uv.len()
        ));
        Ok(())
    }

    fn upload_town_labels(&mut self, bytes: &[u8], visible: bool) {
        self.calls.push(format!("towns {} {visible}", bytes.len()));
    }

    fn upload_road_labels(&mut self, bytes: &[u8], visible: bool) {
        self.calls.push(format!("roads {} {visible}", bytes.len()));
    }

    fn upload_text_labels(&mut self, bytes: &[u8], visible: bool) {
        self.calls
            .push(format!("heights {} {visible}", bytes.len()));
    }

    fn forest_density_upload(&mut self, raster: &ForestDensityRaster<'_>) -> Result<()> {
        self.calls.push(format!(
            "forest {}x{} {}",
            raster.width, raster.height, raster.bins_loaded
        ));
        Ok(())
    }

    fn forest_density_set_params(
        &mut self,
        fill_alpha: f32,
        fill_visible: bool,
        outline_visible: bool,
    ) {
        self.calls.push(format!(
            "forest params {fill_alpha} {fill_visible} {outline_visible}"
        ));
    }

    fn forest_outline_set_stored(&mut self, segments: u32) {
        self.calls.push(format!("forest outline {segments}"));
    }

    fn tex_layer_begin(&mut self, layer: &TextureLayerSpec) -> Result<()> {
        if self.refuse_textures {
            return Err(Error::AssetRejected {
                operation: "tex_layer_begin",
                reason: "no texture layers in this test".into(),
            });
        }
        self.calls.push(format!(
            "begin {} {}x{} {}",
            layer.role, layer.width, layer.height, layer.mip_count
        ));
        Ok(())
    }

    fn tex_layer_write_rgba(&mut self, region: TextureRegion, rgba: &[u8]) -> Result<()> {
        self.calls.push(format!(
            "rgba {} {} {}",
            region.role,
            region.mip,
            rgba.len()
        ));
        Ok(())
    }

    fn tex_layer_write_bitmap(&mut self, region: TextureRegion, image: u32) -> Result<()> {
        self.calls
            .push(format!("bitmap {} {} {image}", region.role, region.mip));
        Ok(())
    }

    fn tex_layer_commit(&mut self, role: u32, opacity: f32, visible: bool) -> Result<()> {
        self.calls
            .push(format!("commit {role} {opacity} {visible}"));
        Ok(())
    }
}

#[test]
fn writes_through_the_shared_handle_reach_the_renderer_cell_in_call_order() {
    let renderer: Rc<RefCell<Option<RecordingSink>>> =
        Rc::new(RefCell::new(Some(RecordingSink::default())));
    let shared: SharedMapAssetSink<u32> = renderer.clone();
    {
        let mut slot = shared.borrow_mut();
        let sink = slot.sink_mut().expect("a booted renderer is a sink");
        sink.set_world_layer_visible("roads", true);
        sink.clear_vector_lane(4);
        sink.upload_icon_lane(WorldGlyphLane::Props, &[0; 20], true);
        let layer = TextureLayerSpec {
            role: 1,
            world_min: [0.0, 0.0],
            world_max: [12_800.0, 12_800.0],
            width: 64,
            height: 32,
            mip_count: 1,
            mode: 3,
        };
        sink.tex_layer_begin(&layer)
            .expect("the recording sink accepts layers");
        let region = TextureRegion {
            role: 1,
            mip: 0,
            x: 0,
            y: 0,
            width: 64,
            height: 32,
        };
        sink.tex_layer_write_bitmap(region, 7)
            .expect("the recording sink accepts writes");
        sink.tex_layer_commit(1, 0.4, true)
            .expect("the recording sink accepts commits");
    }
    let held = renderer.borrow();
    let calls = &held.as_ref().expect("the renderer is still booted").calls;
    assert_eq!(
        calls,
        &[
            "layer roads true",
            "clear 4",
            "icons Props 20 true",
            "begin 1 64x32 1",
            "bitmap 1 0 7",
            "commit 1 0.4 true",
        ],
        "the handle must be the renderer's own cell: every write lands there, in order"
    );
    assert_eq!(
        shared.borrow().sink().map(|sink| sink.stats_json()),
        Some("{\"calls\":6}".to_string()),
        "reads go to the same renderer the writes reached"
    );
}

#[test]
fn a_refused_write_names_the_refused_call() {
    let renderer: Rc<RefCell<Option<RecordingSink>>> = Rc::new(RefCell::new(Some(RecordingSink {
        calls: Vec::new(),
        refuse_textures: true,
    })));
    let shared: SharedMapAssetSink<u32> = renderer;
    let layer = TextureLayerSpec {
        role: 0,
        world_min: [0.0, 0.0],
        world_max: [1.0, 1.0],
        width: 1,
        height: 1,
        mip_count: 1,
        mode: 0,
    };
    let refused = shared
        .borrow_mut()
        .sink_mut()
        .expect("a booted renderer is a sink")
        .tex_layer_begin(&layer)
        .expect_err("this sink refuses texture layers");
    assert_eq!(
        refused.to_string(),
        "the renderer refused tex_layer_begin: no texture layers in this test"
    );
}
