//! **Role:** the icon uniform block layout and its packer, the world-to-anchor conversion of
//! packed icon instances, and the table naming which atlas a sprite lane samples.
//! **Position:** `symbology_layers_gpu`; the glyph atlas, the slot atlas and the icon lanes
//! of this module pack their uniforms and instances through it, and the renderer's boot sizes the
//! icon bind-group layout from [`ICON_UNIFORM_BYTES`] and its compute cull reads
//! [`sprite_atlas_for`].
//! **Signals & state:** none; constants and pure functions.
//! **Invariants:** the uniform block is the UV table (`ATLAS_GLYPH_COUNT` cells of four `f32`)
//! followed by the drag offset (two `f32`) and the pixels-to-metres scale (one `f32`), padded to
//! 16 bytes; a packed icon instance is 20 bytes whose first two `f32` are its position, which the
//! conversion moves from world metres to metres from `ANCHOR`.

use map_coordinates::terrain_frames::ANCHOR;
use map_draw_lanes::lane_roles::LaneRole;
use render_primitives::frame::ids::BindGroupId;
use renderer_core::packet_bindings::{
    BIND_GLYPH_ATLAS, BIND_MOVABLE_SPRITE_ATLAS, BIND_MOVABLE_SPRITE_ATLAS_DRAGGED,
};

/// Bytes of the UV table at the head of the icon uniform block.
pub const ICON_UV_BYTES: usize = render_primitives::draw::instances::ATLAS_GLYPH_COUNT * 16;

/// Bytes of the whole icon uniform block: the UV table plus one 16-byte row.
pub const ICON_UNIFORM_BYTES: u64 = (ICON_UV_BYTES + 16) as u64;

/// Byte offset of the drag offset (`dx`, `dy`) in the icon uniform block.
pub const ICON_DRAG_OFF: usize = ICON_UV_BYTES;

/// Byte offset of the pixels-to-metres scale in the icon uniform block.
pub const ICON_PXM_OFF: usize = ICON_UV_BYTES + 8;

/// Pack the icon uniform block: `uv` (cut to the table's capacity), the drag offset and the
/// pixels-to-metres scale, little-endian.
#[must_use]
pub fn pack_icon_uniforms(uv: &[f32], drag_dx: f32, drag_dy: f32, px_to_m: f32) -> Vec<u8> {
    let mut u_bytes = vec![0u8; ICON_UNIFORM_BYTES as usize];
    for (i, v) in uv.iter().enumerate() {
        let off = i * 4;
        if off + 4 <= ICON_UV_BYTES {
            u_bytes[off..off + 4].copy_from_slice(&v.to_le_bytes());
        }
    }
    u_bytes[ICON_DRAG_OFF..ICON_DRAG_OFF + 4].copy_from_slice(&drag_dx.to_le_bytes());
    u_bytes[ICON_DRAG_OFF + 4..ICON_DRAG_OFF + 8].copy_from_slice(&drag_dy.to_le_bytes());
    u_bytes[ICON_PXM_OFF..ICON_PXM_OFF + 4].copy_from_slice(&px_to_m.to_le_bytes());
    u_bytes
}

/// Move every 20-byte icon instance in `bytes` from world metres to metres from `ANCHOR`, in
/// place.
pub fn convert_icon_world_to_anchor(bytes: &mut [u8]) {
    const STRIDE: usize = 20;
    for chunk in bytes.chunks_exact_mut(STRIDE) {
        let x = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        let y = f32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
        let ax = (f64::from(x) - ANCHOR[0]) as f32;
        let ay = (f64::from(y) - ANCHOR[1]) as f32;
        chunk[0..4].copy_from_slice(&ax.to_le_bytes());
        chunk[4..8].copy_from_slice(&ay.to_le_bytes());
    }
}

/// Which atlas a sprite lane samples at group 2: the slot, cluster, placement preview, vehicle,
/// marker and comment lanes sample the movable-sprite atlas (the slot drag lane with its drag
/// offset applied), every other sprite lane the glyph atlas. The lane uploads and the renderer's
/// compute cull both read this one table.
#[must_use]
pub fn sprite_atlas_for(role: LaneRole) -> BindGroupId {
    match role {
        LaneRole::SlotDrag => BIND_MOVABLE_SPRITE_ATLAS_DRAGGED,

        LaneRole::Slots
        | LaneRole::Clusters
        | LaneRole::SlotPlacePreview
        | LaneRole::MissionVehicles
        | LaneRole::MissionMarkers
        | LaneRole::MissionComments => BIND_MOVABLE_SPRITE_ATLAS,
        _ => BIND_GLYPH_ATLAS,
    }
}

#[cfg(test)]
#[path = "tests/icon_uniforms_tests.rs"]
mod tests;
