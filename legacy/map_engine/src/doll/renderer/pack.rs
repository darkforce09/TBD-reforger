//! Role: pack.
//! Position: `doll/renderer` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Canonical instance stride value.
pub const INSTANCE_STRIDE: usize = 80;

/// Instance streams.
pub struct InstanceStreams {
    /// Bytes.
    pub bytes: Vec<u8>,

    /// N cube.
    pub n_cube: u32,

    /// N cyl.
    pub n_cyl: u32,
}

/// Pack instances.
#[must_use]
pub fn pack_instances(states: &[u8; 14], hover: i32) -> InstanceStreams {
    let all = crate::doll::scene::instances::instances();
    let color_of = |inst: &crate::doll::scene::instances::DollInstance| -> [f32; 4] {
        if inst.region < 0 {
            crate::doll::scene::instances::decor_color()
        } else {
            let idx = usize::try_from(inst.region).unwrap_or(0);
            crate::doll::scene::instances::state_color(
                states
                    .get(idx)
                    .copied()
                    .unwrap_or(crate::doll::scene::instances::STATE_EMPTY),
                inst.region == hover,
            )
        }
    };
    let mut bytes: Vec<u8> = Vec::with_capacity(all.len() * INSTANCE_STRIDE);
    let mut push = |inst: &crate::doll::scene::instances::DollInstance| {
        let model: [f32; 16] = core::array::from_fn(|i| inst.model[i] as f32);
        bytes.extend_from_slice(bytemuck::cast_slice(&model));
        bytes.extend_from_slice(bytemuck::cast_slice(&color_of(inst)));
    };
    let mut n_cube = 0u32;
    let mut n_cyl = 0u32;
    for inst in all
        .iter()
        .filter(|i| i.mesh == crate::doll::scene::instances::MeshKind::Cube)
    {
        push(inst);
        n_cube += 1;
    }
    for inst in all
        .iter()
        .filter(|i| i.mesh == crate::doll::scene::instances::MeshKind::Cylinder)
    {
        push(inst);
        n_cyl += 1;
    }
    InstanceStreams {
        bytes,
        n_cube,
        n_cyl,
    }
}

#[cfg(test)]
#[path = "tests/pack_tests.rs"]
mod tests;
