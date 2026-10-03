//! The icon uniform block layout, the anchor conversion and the sprite atlas table.

use super::{
    ICON_DRAG_OFF, ICON_PXM_OFF, ICON_UNIFORM_BYTES, ICON_UV_BYTES, convert_icon_world_to_anchor,
    pack_icon_uniforms, sprite_atlas_for,
};
use map_coordinates::terrain_frames::ANCHOR;
use map_draw_lanes::lane_roles::LaneRole;
use renderer_core::packet_bindings::{
    BIND_GLYPH_ATLAS, BIND_MOVABLE_SPRITE_ATLAS, BIND_MOVABLE_SPRITE_ATLAS_DRAGGED,
};

fn f32_at(bytes: &[u8], offset: usize) -> f32 {
    let mut word = [0u8; 4];
    word.copy_from_slice(&bytes[offset..offset + 4]);
    f32::from_le_bytes(word)
}

#[test]
fn the_uniform_block_is_the_uv_table_then_one_sixteen_byte_row() {
    assert_eq!(ICON_UNIFORM_BYTES as usize, ICON_UV_BYTES + 16);
    assert_eq!(ICON_DRAG_OFF, ICON_UV_BYTES);
    assert_eq!(ICON_PXM_OFF, ICON_UV_BYTES + 8);
}

#[test]
fn pack_icon_uniforms_writes_the_uv_table_the_drag_offset_and_the_scale() {
    let bytes = pack_icon_uniforms(&[0.25, 0.5, 0.75, 1.0], 3.0, -2.0, 4.0);
    assert_eq!(bytes.len(), ICON_UNIFORM_BYTES as usize);
    assert_eq!(f32_at(&bytes, 0), 0.25);
    assert_eq!(f32_at(&bytes, 12), 1.0);
    assert_eq!(f32_at(&bytes, 16), 0.0, "cells past the table stay zero");
    assert_eq!(f32_at(&bytes, ICON_DRAG_OFF), 3.0);
    assert_eq!(f32_at(&bytes, ICON_DRAG_OFF + 4), -2.0);
    assert_eq!(f32_at(&bytes, ICON_PXM_OFF), 4.0);
}

#[test]
fn pack_icon_uniforms_cuts_a_uv_table_longer_than_the_block() {
    let uv = vec![9.0_f32; ICON_UV_BYTES / 4 + 8];
    let bytes = pack_icon_uniforms(&uv, 1.0, 2.0, 3.0);
    assert_eq!(bytes.len(), ICON_UNIFORM_BYTES as usize);
    assert_eq!(f32_at(&bytes, ICON_UV_BYTES - 4), 9.0);
    assert_eq!(
        f32_at(&bytes, ICON_DRAG_OFF),
        1.0,
        "the overflow never reaches the drag row"
    );
    assert_eq!(f32_at(&bytes, ICON_PXM_OFF), 3.0);
}

#[test]
fn convert_icon_world_to_anchor_moves_only_the_position_of_each_instance() {
    let mut bytes = Vec::new();
    for (x, y) in [
        (ANCHOR[0] as f32 + 10.0, ANCHOR[1] as f32 - 20.0),
        (ANCHOR[0] as f32, ANCHOR[1] as f32),
    ] {
        bytes.extend_from_slice(&x.to_le_bytes());
        bytes.extend_from_slice(&y.to_le_bytes());
        bytes.extend_from_slice(&[7u8; 12]);
    }
    convert_icon_world_to_anchor(&mut bytes);
    assert_eq!(f32_at(&bytes, 0), 10.0);
    assert_eq!(f32_at(&bytes, 4), -20.0);
    assert_eq!(&bytes[8..20], &[7u8; 12]);
    assert_eq!(f32_at(&bytes, 20), 0.0);
    assert_eq!(f32_at(&bytes, 24), 0.0);
}

#[test]
fn sprite_atlas_for_routes_the_mission_lanes_to_the_movable_sprite_atlas() {
    assert_eq!(
        sprite_atlas_for(LaneRole::SlotDrag),
        BIND_MOVABLE_SPRITE_ATLAS_DRAGGED
    );
    for role in [
        LaneRole::Slots,
        LaneRole::Clusters,
        LaneRole::SlotPlacePreview,
        LaneRole::MissionVehicles,
        LaneRole::MissionMarkers,
        LaneRole::MissionComments,
    ] {
        assert_eq!(
            sprite_atlas_for(role),
            BIND_MOVABLE_SPRITE_ATLAS,
            "{role:?}"
        );
    }
    for role in [
        LaneRole::WorldTrees,
        LaneRole::WorldProps,
        LaneRole::WorldBadges,
    ] {
        assert_eq!(sprite_atlas_for(role), BIND_GLYPH_ATLAS, "{role:?}");
    }
}
