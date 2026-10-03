//! The paper doll's instance stream: every part's model matrix and colour as GPU bytes.
//!
//! **Role:** packs the soldier's parts ([`paper_doll_scene::soldier_parts::instances`]) into one
//! byte stream of [`INSTANCE_STRIDE`] bytes a part, coloured by the region states and the hovered
//! region ([`pack_instances`]).
//! **Position:** the GPU-free half of `paper_doll_renderer`; the renderer uploads the stream into
//! its instance buffer on creation and on every state or hover change, and the self-check packs
//! its own probe states.
//! **Signals & state:** none; pure functions.
//! **Invariants:** a part is its 16 `f32` model matrix then its 4 `f32` colour (80 bytes, the
//! five `Float32x4` instance attributes of the pipeline and `doll.wgsl`); every cube precedes every
//! cylinder, so one buffer serves both instanced draws at two offsets.

use paper_doll_scene::soldier_parts::{
    DollInstance, MeshKind, REGION_COUNT, STATE_EMPTY, decor_color, instances, state_color,
};

/// The bytes of one part in the instance stream: a 4x4 `f32` matrix and an RGBA `f32` colour.
pub const INSTANCE_STRIDE: usize = 80;

/// The packed instance stream and how many of its parts each mesh draws.
pub struct InstanceStreams {
    /// Every part, cubes first, [`INSTANCE_STRIDE`] bytes each.
    pub bytes: Vec<u8>,

    /// The cube parts at the start of the stream.
    pub cube_instances: u32,

    /// The cylinder parts after the cubes.
    pub cylinder_instances: u32,
}

/// Pack the soldier's parts with each region coloured by its byte in `states` (in
/// [`paper_doll_scene::soldier_parts::REGION_KEYS`] order) and the region `hover` (or -1 for
/// none) lifted; body decor takes [`decor_color`].
#[must_use]
pub fn pack_instances(states: &[u8; REGION_COUNT], hover: i32) -> InstanceStreams {
    let all = instances();
    let color_of = |inst: &DollInstance| -> [f32; 4] {
        if inst.region < 0 {
            decor_color()
        } else {
            let idx = usize::try_from(inst.region).unwrap_or(0);
            state_color(
                states.get(idx).copied().unwrap_or(STATE_EMPTY),
                inst.region == hover,
            )
        }
    };
    let mut bytes: Vec<u8> = Vec::with_capacity(all.len() * INSTANCE_STRIDE);
    let mut push = |inst: &DollInstance| {
        let model: [f32; 16] = core::array::from_fn(|i| inst.model[i] as f32);
        bytes.extend_from_slice(bytemuck::cast_slice(&model));
        bytes.extend_from_slice(bytemuck::cast_slice(&color_of(inst)));
    };
    let mut cube_instances = 0u32;
    let mut cylinder_instances = 0u32;
    for inst in all.iter().filter(|i| i.mesh == MeshKind::Cube) {
        push(inst);
        cube_instances += 1;
    }
    for inst in all.iter().filter(|i| i.mesh == MeshKind::Cylinder) {
        push(inst);
        cylinder_instances += 1;
    }
    InstanceStreams {
        bytes,
        cube_instances,
        cylinder_instances,
    }
}

#[cfg(test)]
#[path = "tests/instance_packing_tests.rs"]
mod tests;
