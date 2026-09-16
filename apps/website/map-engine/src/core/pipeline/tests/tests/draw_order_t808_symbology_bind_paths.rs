//! Role: draw order t808 symbology bind paths.
//! Position: `core/pipeline/tests/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

const ENGINE: &str = concat!(
    include_str!("../../../../renderers/engine/mod.rs"),
    "\n",
    include_str!("../../../../symbology/instances/bridge_1.rs"),
    "\n",
    include_str!("../../../../symbology/instances/bridge_2.rs"),
    "\n",
    include_str!("../../../../symbology/instances/bridge_3.rs"),
    "\n",
    include_str!("../../../context/state.rs"),
    "\n",
    include_str!("../../../context/device_1.rs"),
    "\n",
    include_str!("../../../context/device_2.rs"),
    "\n",
    include_str!("../../../../diagnostics/timing/gpu.rs"),
    "\n",
    include_str!("../../../context/viewport.rs"),
    "\n",
    include_str!("../../../../terrain/satellite/textures.rs"),
    "\n",
    include_str!("../../../../renderers/primitives/hairlines.rs"),
    "\n",
    include_str!("../../../../renderers/primitives/vector_lines.rs"),
    "\n",
    include_str!("../../../../symbology/instances/lanes.rs"),
    "\n",
    include_str!("../../../../renderers/engine/lifecycle.rs"),
    "\n",
    include_str!("../../../../renderers/text/lanes.rs"),
    "\n",
    include_str!("../../../../symbology/atlas/gpu.rs"),
    "\n",
    // T-0xx Phase 1D: `renderers/batching/batch.rs` left this list because the file is
    // gone — `Batch`/`BatchPayload`/`IndirectIcon` are now `website-graphics-engine`'s
    // `frame::{DrawBatch, DrawPayload, IndirectDraw}`. It held type definitions, never an
    // `impl RenderEngine` body, so no `body(sig)` lookup below resolved into it; the suite
    // was run with it removed to prove that rather than assume it.
    // T-0xx Phase 1C: the six `renderers/pipelines/*.rs` members left for
    // `website-graphics-engine`. They held free `create_*_pipeline` fns, never an
    // `impl RenderEngine` body, so no `body(sig)` lookup below resolved into them —
    // proven by running this suite with them removed. A cross-crate `include_str!`
    // would have kept the bytes at the cost of making this crate's tests break on the
    // renderer's internal layout, which is the coupling the split exists to remove.
    include_str!("../../../../renderers/batching/encoder.rs"),
    "\n",
    include_str!("../../../culling/engine.rs"),
    "\n",
    include_str!("../../../../diagnostics/bench/frame_1.rs"),
    "\n",
    include_str!("../../../../diagnostics/bench/frame_2.rs"),
    "\n",
    include_str!("../../../../diagnostics/readback/scene.rs"),
    "\n",
    include_str!("../../../../environment/vegetation/buffers.rs"),
    "\n",
    include_str!("../../../../spatial/terrain_los/overlay.rs"),
    "\n",
    include_str!("../../../context/preferences.rs"),
    "\n",
    include_str!("../../../../environment/buildings/buffers.rs"),
    "\n",
    include_str!("../../../../renderers/primitives/selection.rs"),
    "\n",
    include_str!("../../../../diagnostics/readback/texture.rs"),
    "\n",
    include_str!("../../../../diagnostics/readback/world_building.rs"),
    "\n",
    include_str!("../../../../diagnostics/readback/sea_band.rs"),
    "\n",
    include_str!("../../../../diagnostics/readback/road_centerline.rs"),
    "\n",
    include_str!("../../../../diagnostics/readback/tree_glyph.rs"),
    "\n",
    include_str!("../../../../diagnostics/readback/text.rs"),
    "\n",
    include_str!("../../../../diagnostics/readback/marquee.rs"),
    "\n",
    include_str!("../../../../diagnostics/readback/compute_cull.rs")
);

fn body(sig: &str) -> String {
    let start = ENGINE
        .find(sig)
        .unwrap_or_else(|| panic!("T-808: engine.rs has no `{sig}`"));
    assert!(
        !ENGINE[start + sig.len()..].contains(sig),
        "T-808: `{sig}` is not unique in engine.rs — the extractor would pin the wrong body"
    );
    let after = &ENGINE[start..];
    let brace = after.find('{').expect("body");
    let mut depth = 0usize;
    let mut end = brace;
    for (i, ch) in after[brace..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = brace + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    after[..end].to_string()
}

#[test]
fn ensure_slot_atlas_widens_and_records_the_symbology_base() {
    let b = body("pub fn ensure_slot_atlas");
    assert!(
        b.contains("extend_atlas_with_unit_glyphs"),
        "T-808: ensure_slot_atlas must widen the strip via \
             slots_gpu::extend_atlas_with_unit_glyphs; body:\n{b}"
    );
    assert!(
        b.contains("symbology_base = Some(wide.base_cells)"),
        "T-808: ensure_slot_atlas must record the runtime symbology base; body:\n{b}"
    );
    assert!(
        b.contains("symbology_base = None"),
        "T-808: an unparseable strip must CLEAR symbology_base, not keep a stale one; \
             body:\n{b}"
    );
    assert!(
        b.contains("atlas_ready = true"),
        "T-808: ensure_slot_atlas must still arm the slot bridge; body:\n{b}"
    );
}

#[test]
fn slots_bind_symbology_keeps_the_role_and_heading_columns() {
    let b = body("pub fn slots_bind_symbology");
    assert!(
        b.contains("last_roles = roles"),
        "T-808: slots_bind_symbology must store the ROLE column; body:\n{b}"
    );
    assert!(
        b.contains("last_headings = headings_deg"),
        "T-808: slots_bind_symbology must store the HEADING column; body:\n{b}"
    );
    assert!(
        b.contains("rematerialize_slot_lane"),
        "T-808: slots_bind_symbology must re-pack the slot lane from the new columns; \
             body:\n{b}"
    );

    let soa = body("pub fn slots_bind_soa");
    assert!(
        soa.contains("slots_bind_symbology"),
        "T-808: slots_bind_soa must delegate to slots_bind_symbology; body:\n{soa}"
    );
}

#[test]
fn vehicles_bind_symbology_uploads_its_lane_and_falls_back_without_symbology() {
    let b = body("pub fn vehicles_bind_symbology");
    assert!(
        b.contains("MissionVehicles"),
        "T-808: vehicles_bind_symbology must upload LaneRole::MissionVehicles; body:\n{b}"
    );
    assert!(
        b.contains("pack_vehicle_symbology"),
        "T-808: vehicles_bind_symbology must pack via slots_gpu::pack_vehicle_symbology; \
             body:\n{b}"
    );
    assert!(
        b.contains("symbology_base") && b.contains("self.vehicles_bind(xy)"),
        "T-808: vehicles_bind_symbology must fall back to the disc lane when the atlas \
             carries no symbology cells; body:\n{b}"
    );
    assert!(
        !b.contains("last_ids") && !b.contains("slots_bind_soa"),
        "T-808: vehicles_bind_symbology must not enter the slot pick / SoA bridge; body:\n{b}"
    );
}

#[test]
fn refresh_comment_lane_repacks_from_cache_and_is_actually_called() {
    let b = body("fn refresh_comment_lane(&mut self)");
    assert!(
        b.contains("comment_xy"),
        "T-808: refresh_comment_lane must re-pack from the cached comment_xy; body:\n{b}"
    );
    assert!(
        b.contains("self.comments_bind(&xy)"),
        "T-808: refresh_comment_lane must re-pack through comments_bind; body:\n{b}"
    );
    for feed in ["pub fn set_selection", "fn sync_slot_zoom_uniform"] {
        let f = body(feed);
        assert!(
            f.contains("refresh_comment_lane"),
            "T-808: `{feed}` must refresh the comment lane; body:\n{f}"
        );
    }
}
