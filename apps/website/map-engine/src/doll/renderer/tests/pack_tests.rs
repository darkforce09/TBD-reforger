//! Role: pack tests.
//! Position: `doll/renderer/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::doll::scene::model as doll;
use crate::doll::scene::model::MeshKind;

use crate::doll::renderer::pack::*;

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
        .filter(|i| i.mesh == MeshKind::Cylinder)
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

    let all = doll::instances();
    let ordered: Vec<i32> = all
        .iter()
        .filter(|i| i.mesh == MeshKind::Cube)
        .chain(all.iter().filter(|i| i.mesh == MeshKind::Cylinder))
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
        .filter(|i| i.mesh == MeshKind::Cube)
        .chain(all.iter().filter(|i| i.mesh == MeshKind::Cylinder))
        .map(|i| i.region)
        .collect();
    for (slot, region) in ordered.iter().enumerate() {
        let expect_change = *region == i32::try_from(vest).unwrap();
        let changed = color_at(&plain, slot) != color_at(&hovered, slot);
        assert_eq!(changed, expect_change, "instance {slot} (region {region})");
    }
}
