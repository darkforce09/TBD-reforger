//! T-154 — pure instance packing for the doll pipeline (no wgpu/web types; native-tested
//! byte layout, Class R style). One 80-byte instance per part: column-major model mat4 as
//! f32 (64 B) + RGBA color f32 (16 B). Cubes stream first, cylinders after — the render
//! pass selects each mesh's range via buffer slicing (WebGL2 has no `first_instance`).

use map_engine_core::doll;

pub const INSTANCE_STRIDE: usize = 80;

pub struct InstanceStreams {
    pub bytes: Vec<u8>,
    pub n_cube: u32,
    pub n_cyl: u32,
}

#[must_use]
pub fn pack_instances(states: &[u8; 14], hover: i32) -> InstanceStreams {
    let all = doll::instances();
    let color_of = |inst: &doll::DollInstance| -> [f32; 4] {
        if inst.region < 0 {
            doll::decor_color()
        } else {
            let idx = usize::try_from(inst.region).unwrap_or(0);
            doll::state_color(
                states.get(idx).copied().unwrap_or(doll::STATE_EMPTY),
                inst.region == hover,
            )
        }
    };
    let mut bytes: Vec<u8> = Vec::with_capacity(all.len() * INSTANCE_STRIDE);
    let mut push = |inst: &doll::DollInstance| {
        let model: [f32; 16] = core::array::from_fn(|i| inst.model[i] as f32);
        bytes.extend_from_slice(bytemuck::cast_slice(&model));
        bytes.extend_from_slice(bytemuck::cast_slice(&color_of(inst)));
    };
    let mut n_cube = 0u32;
    let mut n_cyl = 0u32;
    for inst in all.iter().filter(|i| i.mesh == doll::MeshKind::Cube) {
        push(inst);
        n_cube += 1;
    }
    for inst in all.iter().filter(|i| i.mesh == doll::MeshKind::Cylinder) {
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
mod tests {
    use super::*;

    fn color_at(streams: &InstanceStreams, instance: usize) -> [f32; 4] {
        let base = instance * INSTANCE_STRIDE + 64;
        let mut out = [0f32; 4];
        out.copy_from_slice(bytemuck::cast_slice(&streams.bytes[base..base + 16]));
        out
    }

    #[test]
    fn byte_layout_golden() {
        let s = pack_instances(&[doll::STATE_EMPTY; 14], -1);
        let total = doll::instances().len();
        assert_eq!(s.bytes.len(), total * INSTANCE_STRIDE);
        assert_eq!((s.n_cube + s.n_cyl) as usize, total);
        assert_eq!(s.n_cyl, 1, "exactly the launcher tube is a cylinder");
    }

    #[test]
    fn cubes_stream_before_cylinders_and_launcher_is_the_cylinder() {
        let all = doll::instances();
        let launcher = doll::REGION_KEYS
            .iter()
            .position(|k| *k == "launcher")
            .unwrap();
        let cyl_regions: Vec<i32> = all
            .iter()
            .filter(|i| i.mesh == doll::MeshKind::Cylinder)
            .map(|i| i.region)
            .collect();
        assert_eq!(cyl_regions, vec![i32::try_from(launcher).unwrap()]);
    }

    #[test]
    fn state_flip_rewrites_exactly_the_region_colors() {
        let empty = pack_instances(&[doll::STATE_EMPTY; 14], -1);
        let mut states = [doll::STATE_EMPTY; 14];
        let helmet = doll::REGION_KEYS
            .iter()
            .position(|k| *k == "headCover")
            .unwrap();
        states[helmet] = doll::STATE_ACTIVE;
        let flipped = pack_instances(&states, -1);

        // Instance order is deterministic; find helmet's packed slot by scanning regions
        // in the same cube-then-cylinder order the packer uses.
        let all = doll::instances();
        let ordered: Vec<i32> = all
            .iter()
            .filter(|i| i.mesh == doll::MeshKind::Cube)
            .chain(all.iter().filter(|i| i.mesh == doll::MeshKind::Cylinder))
            .map(|i| i.region)
            .collect();
        for (slot, region) in ordered.iter().enumerate() {
            let expect_change = *region == i32::try_from(helmet).unwrap();
            let changed = color_at(&empty, slot) != color_at(&flipped, slot);
            assert_eq!(changed, expect_change, "instance {slot} (region {region})");
        }
        let helmet_slot = ordered
            .iter()
            .position(|r| *r == i32::try_from(helmet).unwrap())
            .unwrap();
        assert_eq!(
            color_at(&flipped, helmet_slot),
            doll::state_color(doll::STATE_ACTIVE, false)
        );
    }

    #[test]
    fn hover_flip_rewrites_exactly_the_hovered_region() {
        let states = [doll::STATE_EMPTY; 14];
        let plain = pack_instances(&states, -1);
        let vest = doll::REGION_KEYS.iter().position(|k| *k == "vest").unwrap();
        let hovered = pack_instances(&states, i32::try_from(vest).unwrap());
        let all = doll::instances();
        let ordered: Vec<i32> = all
            .iter()
            .filter(|i| i.mesh == doll::MeshKind::Cube)
            .chain(all.iter().filter(|i| i.mesh == doll::MeshKind::Cylinder))
            .map(|i| i.region)
            .collect();
        for (slot, region) in ordered.iter().enumerate() {
            let expect_change = *region == i32::try_from(vest).unwrap();
            let changed = color_at(&plain, slot) != color_at(&hovered, slot);
            assert_eq!(changed, expect_change, "instance {slot} (region {region})");
        }
    }
}
