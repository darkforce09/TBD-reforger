//! Role: draw order t748 comments bind pick bridge.
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
    include_str!("../../../../renderers/batching/batch.rs"),
    "\n",
    include_str!("../../../../renderers/pipelines/quad.rs"),
    "\n",
    include_str!("../../../../renderers/pipelines/textured.rs"),
    "\n",
    include_str!("../../../../renderers/pipelines/vector.rs"),
    "\n",
    include_str!("../../../../renderers/pipelines/text.rs"),
    "\n",
    include_str!("../../../../renderers/pipelines/icon.rs"),
    "\n",
    include_str!("../../../../renderers/pipelines/building.rs"),
    "\n",
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

fn comments_bind_body() -> String {
    let sig = "pub fn comments_bind";
    let start = ENGINE
        .find(sig)
        .unwrap_or_else(|| panic!("T-748: missing {sig}"));
    let after = &ENGINE[start..];
    let brace = after.find('{').expect("comments_bind body");
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
fn comments_bind_body_does_not_touch_last_ids() {
    let body = comments_bind_body();
    assert!(
        !body.contains("last_ids"),
        "T-748: comments_bind must not touch pick-bridge last_ids; body:\n{body}"
    );
    assert!(
        body.contains("MissionComments"),
        "T-748: comments_bind must upload LaneRole::MissionComments; body:\n{body}"
    );
    assert!(
        !body.contains("slots_bind_soa"),
        "T-748: comments_bind must not call slots_bind_soa; body:\n{body}"
    );
}
