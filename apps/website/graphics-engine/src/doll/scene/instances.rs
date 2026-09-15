//! Role: instances.
//! Position: `doll/scene` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::camera::math::glmat4::identity;
use crate::camera::math::glmat4::multiply;
use crate::camera::math::glmat4::scale_in_place;
use crate::camera::math::glmat4::translate_in_place;

/// Clickable regions, RAIL order (contract with `loadout/arsenalDollModel.ts`).
pub const REGION_KEYS: [&str; 14] = [
    "primary",
    "optic",
    "magazine",
    "launcher",
    "handgun",
    "throwable",
    "headCover",
    "jacket",
    "vest",
    "armoredVest",
    "backpack",
    "handwear",
    "pants",
    "boots",
];

/// Canonical state empty value.
pub const STATE_EMPTY: u8 = 0;

/// Canonical state equipped value.
pub const STATE_EQUIPPED: u8 = 1;

/// Canonical state active value.
pub const STATE_ACTIVE: u8 = 2;

/// Region index marking non-clickable body decor (head, neck).
pub const DECOR: i32 = -1;

/// Mesh kind.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MeshKind {
    /// Cube.
    Cube,

    /// Cylinder.
    Cylinder,
}

/// One drawn (and pickable, when `region >= 0`) part of the soldier.
pub struct DollInstance {
    /// Index into [`REGION_KEYS`], or [`DECOR`].
    pub region: i32,

    /// Mesh.
    pub mesh: MeshKind,

    /// Column-major model matrix (f64; cast to f32 at the GPU boundary).
    pub model: [f64; 16],
}

/// Rotate z.
pub(crate) fn rotate_z(theta: f64) -> [f64; 16] {
    let (s, c) = theta.sin_cos();
    let mut m = identity();
    m[0] = c;
    m[1] = s;
    m[4] = -s;
    m[5] = c;
    m
}

/// Trs.
pub(crate) fn trs(t: [f64; 3], rz: f64, s: [f64; 3]) -> [f64; 16] {
    let mut m = identity();
    translate_in_place(&mut m, t);
    let mut m = multiply(&m, &rotate_z(rz));
    scale_in_place(&mut m, s);
    m
}

/// The soldier: body decor + one-or-more instances per clickable region. Proportions are schematic (~1.8 m frame); the rifle hangs diagonally across the chest with the optic and magazine as its own clickable boxes (the ACE interaction).
#[must_use]
pub fn instances() -> Vec<DollInstance> {
    let mut out = Vec::new();
    let mut push = |region: i32, mesh: MeshKind, model: [f64; 16]| {
        out.push(DollInstance {
            region,
            mesh,
            model,
        });
    };
    let cube = MeshKind::Cube;
    let cyl = MeshKind::Cylinder;

    push(DECOR, cube, trs([0.0, 1.73, 0.0], 0.0, [0.22, 0.22, 0.22]));
    push(DECOR, cube, trs([0.0, 1.585, 0.0], 0.0, [0.10, 0.07, 0.10]));

    let region = |key: &str| -> i32 {
        REGION_KEYS
            .iter()
            .position(|k| *k == key)
            .map_or(DECOR, |i| i32::try_from(i).unwrap_or(DECOR))
    };

    push(
        region("headCover"),
        cube,
        trs([0.0, 1.82, 0.0], 0.0, [0.28, 0.14, 0.28]),
    );

    push(
        region("jacket"),
        cube,
        trs([0.0, 1.25, 0.0], 0.0, [0.42, 0.55, 0.24]),
    );
    push(
        region("jacket"),
        cube,
        trs([-0.27, 1.18, 0.0], 0.0, [0.10, 0.50, 0.11]),
    );
    push(
        region("jacket"),
        cube,
        trs([0.27, 1.18, 0.0], 0.0, [0.10, 0.50, 0.11]),
    );
    push(
        region("vest"),
        cube,
        trs([0.0, 1.28, 0.15], 0.0, [0.30, 0.28, 0.06]),
    );
    push(
        region("armoredVest"),
        cube,
        trs([0.0, 1.27, 0.0], 0.0, [0.46, 0.38, 0.28]),
    );
    push(
        region("backpack"),
        cube,
        trs([-0.04, 1.22, -0.27], 0.0, [0.34, 0.46, 0.18]),
    );
    push(
        region("handwear"),
        cube,
        trs([-0.27, 0.88, 0.0], 0.0, [0.11, 0.11, 0.11]),
    );
    push(
        region("handwear"),
        cube,
        trs([0.27, 0.88, 0.0], 0.0, [0.11, 0.11, 0.11]),
    );

    push(
        region("pants"),
        cube,
        trs([0.0, 0.90, 0.0], 0.0, [0.40, 0.16, 0.22]),
    );
    push(
        region("pants"),
        cube,
        trs([-0.11, 0.48, 0.0], 0.0, [0.16, 0.72, 0.18]),
    );
    push(
        region("pants"),
        cube,
        trs([0.11, 0.48, 0.0], 0.0, [0.16, 0.72, 0.18]),
    );
    push(
        region("boots"),
        cube,
        trs([-0.11, 0.08, 0.03], 0.0, [0.17, 0.16, 0.28]),
    );
    push(
        region("boots"),
        cube,
        trs([0.11, 0.08, 0.03], 0.0, [0.17, 0.16, 0.28]),
    );

    push(
        region("handgun"),
        cube,
        trs([0.26, 0.92, 0.10], 0.0, [0.10, 0.16, 0.07]),
    );
    push(
        region("throwable"),
        cube,
        trs([-0.24, 0.95, 0.12], 0.0, [0.10, 0.12, 0.08]),
    );
    push(
        region("launcher"),
        cyl,
        trs([0.05, 1.20, -0.31], 2.53, [0.10, 0.85, 0.10]),
    );

    let rifle = {
        let mut m = identity();
        translate_in_place(&mut m, [0.0, 1.02, 0.20]);
        multiply(&m, &rotate_z(-0.62))
    };
    let part = |local: [f64; 16]| multiply(&rifle, &local);
    push(
        region("primary"),
        cube,
        part(trs([0.0, 0.0, 0.0], 0.0, [0.86, 0.09, 0.07])),
    );
    push(
        region("optic"),
        cube,
        part(trs([0.10, 0.095, 0.0], 0.0, [0.16, 0.10, 0.06])),
    );
    push(
        region("magazine"),
        cube,
        part(trs([-0.06, -0.13, 0.0], 0.0, [0.07, 0.17, 0.06])),
    );

    out
}

/// State color.
#[must_use]
pub fn state_color(state: u8, hovered: bool) -> [f32; 4] {
    let base = match state {
        STATE_ACTIVE => [0.678, 0.776, 1.0, 1.0],
        STATE_EQUIPPED => [0.40, 0.51, 0.74, 1.0],
        _ => [0.165, 0.185, 0.235, 1.0],
    };
    if !hovered {
        return base;
    }
    [
        (base[0] * 1.22).min(1.0),
        (base[1] * 1.22).min(1.0),
        (base[2] * 1.22).min(1.0),
        1.0,
    ]
}

/// Body-decor color (head/neck) — darker than any state so gear reads on top.
#[must_use]
pub fn decor_color() -> [f32; 4] {
    [0.125, 0.14, 0.175, 1.0]
}

/// Canonical clear color value.
pub const CLEAR_COLOR: [f64; 4] = [0.52, 0.56, 0.63, 1.0];
