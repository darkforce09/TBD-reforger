use super::*;
use crate::blueprint::synth;

#[test]
fn box_room_yields_single_ground_slab() {
    let d = synth::box_room(6.0, 4.0, 2.6, 0.15);
    let m = d.meta();
    let p = Params {
        min_floor_y: -0.5 - m.origin[1],
        ..Default::default()
    };
    let v = analyze(&d.y_down, m.dims, m.cell, m.span[1], &p);
    assert_eq!(
        v.floors.len(),
        1,
        "floors: {:?} slabs: {:?} eave {}",
        v.floors,
        v.slabs,
        v.eave
    );
    let local_floor = m.origin[1] + v.floors[0];
    assert!(
        local_floor.abs() < 0.15,
        "ground slab near local y=0, got {local_floor}"
    );
    assert!(
        (m.origin[1] + v.eave - 2.6).abs() < 0.2,
        "flat roof reads as eave~2.6"
    );
    assert!(v.chimney.is_none());
}

#[test]
fn gable_slope_field_flags_roof_planes() {
    let d = synth::gable_box(6.0, 4.0, 2.6, 4.2, 0.15);
    let m = d.meta();
    let p = Params {
        min_floor_y: -0.5 - m.origin[1],
        ..Default::default()
    };
    let v = analyze(&d.y_down, m.dims, m.cell, m.span[1], &p);
    // Mid-span roof cells slope ~ (ridge-eave)/(depth/2) = 1.6/2.0 = 0.8.
    let (ix, iz) = (m.dims[0] / 2, m.dims[2] / 4);
    let s = v.slope_at(ix, iz);
    assert!(
        s > 0.25 && s < 4.0,
        "roof plane slope in veto band, got {s}"
    );
    assert!(
        v.ridge > v.eave + 1.0,
        "gable ridge above eave: {} vs {}",
        v.ridge,
        v.eave
    );
}
