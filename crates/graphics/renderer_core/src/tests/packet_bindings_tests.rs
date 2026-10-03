//! The packet binding ids: dense pipeline slots, distinct fixed bind slots below the texture
//! base, and a texture slot that is a pure offset of its lane.

use crate::packet_bindings::{
    BIND_CAMERA, BIND_GLYPH_ATLAS, BIND_MOVABLE_SPRITE_ATLAS, BIND_MOVABLE_SPRITE_ATLAS_DRAGGED,
    BIND_TEX_BASE, BIND_TEXT_ATLAS, PIPE_DENSITY, PIPE_ICON, PIPE_ICON_STORAGE32, PIPE_LINE,
    PIPE_ORIENTED_QUAD, PIPE_POLYGON, PIPE_QUAD, PIPE_TEXT, PIPE_TEXTURED, PIPELINE_SLOTS,
    tex_bind_id,
};
use render_primitives::frame::ids::{BindGroupId, LaneId};

#[test]
fn pipeline_ids_fill_every_slot_exactly_once() {
    let mut ids: Vec<usize> = [
        PIPE_QUAD,
        PIPE_TEXTURED,
        PIPE_DENSITY,
        PIPE_LINE,
        PIPE_ORIENTED_QUAD,
        PIPE_POLYGON,
        PIPE_ICON,
        PIPE_TEXT,
        PIPE_ICON_STORAGE32,
    ]
    .iter()
    .map(|id| usize::from(id.0))
    .collect();
    ids.sort_unstable();
    assert_eq!(ids, (0..PIPELINE_SLOTS).collect::<Vec<_>>());
}

#[test]
fn fixed_bind_ids_are_dense_below_the_texture_base() {
    let mut ids: Vec<u16> = [
        BIND_CAMERA,
        BIND_GLYPH_ATLAS,
        BIND_TEXT_ATLAS,
        BIND_MOVABLE_SPRITE_ATLAS,
        BIND_MOVABLE_SPRITE_ATLAS_DRAGGED,
    ]
    .iter()
    .map(|id| id.0)
    .collect();
    ids.sort_unstable();
    assert_eq!(ids, (0..BIND_TEX_BASE).collect::<Vec<_>>());
    assert_eq!(
        BIND_CAMERA,
        BindGroupId(0),
        "group 0 is the camera for every draw"
    );
}

#[test]
fn a_lane_texture_slot_is_the_base_plus_the_lane() {
    assert_eq!(tex_bind_id(LaneId(0)), BindGroupId(BIND_TEX_BASE));
    assert_eq!(tex_bind_id(LaneId(94)), BindGroupId(BIND_TEX_BASE + 94));
    let slots: Vec<BindGroupId> = (0..96).map(|l| tex_bind_id(LaneId(l))).collect();
    for (i, a) in slots.iter().enumerate() {
        assert!(
            a.0 >= BIND_TEX_BASE,
            "a texture slot never reuses a fixed slot"
        );
        assert!(
            slots[i + 1..].iter().all(|b| b != a),
            "two lanes never share a slot"
        );
    }
}
