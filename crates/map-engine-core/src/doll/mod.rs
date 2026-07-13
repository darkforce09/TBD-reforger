//! T-154 arsenal doll — pure scene/camera/pick policy for the 3D soldier (D5: Rust owns the
//! doll; TS only forwards pointer events and a 14-byte state array).
//!
//! Coordinate system: meters, Y-up, origin at the feet center, soldier faces +Z at yaw 0.
//! The clickable region list mirrors the frontend `RAIL_REGIONS` order EXACTLY — the TS
//! parity test asserts `doll_region_keys()` against it, and `set_states`/pick indexes are
//! positions in this array. Meshes are unit primitives (cube/cylinder centered at the
//! origin, extent 0.5) instanced by per-part model matrices; picking runs the same
//! matrices through inverse-model ray/slab tests, so the clickable volume IS the drawn volume.

use crate::camera::glmat4::{
    identity, invert, look_at, multiply, perspective_no, scale_in_place, transform_vector,
    translate_in_place,
};

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

pub const STATE_EMPTY: u8 = 0;
pub const STATE_EQUIPPED: u8 = 1;
pub const STATE_ACTIVE: u8 = 2;

/// Region index marking non-clickable body decor (head, neck).
pub const DECOR: i32 = -1;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MeshKind {
    Cube,
    Cylinder,
}

/// One drawn (and pickable, when `region >= 0`) part of the soldier.
pub struct DollInstance {
    /// Index into [`REGION_KEYS`], or [`DECOR`].
    pub region: i32,
    pub mesh: MeshKind,
    /// Column-major model matrix (f64; cast to f32 at the GPU boundary).
    pub model: [f64; 16],
}

/// Rotation about Z (column-major), local to the doll (not a gl-matrix mirror — the doll has
/// no JS oracle; plain f64 composition is the policy here).
fn rotate_z(theta: f64) -> [f64; 16] {
    let (s, c) = theta.sin_cos();
    let mut m = identity();
    m[0] = c;
    m[1] = s;
    m[4] = -s;
    m[5] = c;
    m
}

/// `T(t) · Rz(rz) · S(s)` — the standard part transform.
fn trs(t: [f64; 3], rz: f64, s: [f64; 3]) -> [f64; 16] {
    let mut m = identity();
    translate_in_place(&mut m, t);
    let mut m = multiply(&m, &rotate_z(rz));
    scale_in_place(&mut m, s);
    m
}

/// The soldier: body decor + one-or-more instances per clickable region.
/// Proportions are schematic (~1.8 m frame); the rifle hangs diagonally across the chest
/// with the optic and magazine as its own clickable boxes (the ACE interaction).
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

    // Body decor (not clickable): head + neck.
    push(DECOR, cube, trs([0.0, 1.73, 0.0], 0.0, [0.22, 0.22, 0.22]));
    push(DECOR, cube, trs([0.0, 1.585, 0.0], 0.0, [0.10, 0.07, 0.10]));

    let region = |key: &str| -> i32 {
        REGION_KEYS
            .iter()
            .position(|k| *k == key)
            .map_or(DECOR, |i| i32::try_from(i).unwrap_or(DECOR))
    };

    // Wear.
    push(
        region("headCover"),
        cube,
        trs([0.0, 1.82, 0.0], 0.0, [0.28, 0.14, 0.28]),
    );
    // jacket: torso + both arms.
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
    // pants: pelvis + both legs.
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

    // Belt kit + launcher.
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

    // Rifle: shared frame, diagonal across the chest front; receiver + optic + magazine.
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

/// State → opaque RGBA (no blending; "empty" is a dim opaque tone, not translucent —
/// translucency with a depth buffer would need sorted draws for nothing).
#[must_use]
pub fn state_color(state: u8) -> [f32; 4] {
    match state {
        STATE_ACTIVE => [0.678, 0.776, 1.0, 1.0], // Aegis primary, full
        STATE_EQUIPPED => [0.40, 0.51, 0.74, 1.0], // primary-tinted (0.51: unorm8 tie-safe)
        _ => [0.165, 0.185, 0.235, 1.0],          // dim slate (empty)
    }
}

/// Body-decor color (head/neck) — darker than any state so gear reads on top.
#[must_use]
pub fn decor_color() -> [f32; 4] {
    [0.125, 0.14, 0.175, 1.0]
}

/// Doll backdrop clear color (Aegis dark surface).
pub const CLEAR_COLOR: [f64; 4] = [0.055, 0.062, 0.078, 1.0];

// ── camera ────────────────────────────────────────────────────────────────────

const FOVY: f64 = 0.6109; // ~35°
const NEAR: f64 = 0.1;
const FAR: f64 = 100.0;
const ORBIT_CENTER: [f64; 3] = [0.0, 1.02, 0.0];
const ORBIT_DIST: f64 = 3.3;
const ORBIT_HEIGHT: f64 = 1.45;

/// GL→WebGPU clip-space z remap (same constant as the ortho camera; wgpu presents WebGPU
/// clip conventions on both backends).
const Z01: [f64; 16] = [
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 0.5, 0.0, //
    0.0, 0.0, 0.5, 1.0,
];

fn view(yaw: f64) -> [f64; 16] {
    let eye = [
        ORBIT_CENTER[0] + yaw.sin() * ORBIT_DIST,
        ORBIT_HEIGHT,
        ORBIT_CENTER[2] + yaw.cos() * ORBIT_DIST,
    ];
    look_at(eye, ORBIT_CENTER, [0.0, 1.0, 0.0])
}

/// `P · V` in GL clip conventions (pick path — unproject uses NDC z ∈ [-1, 1]).
#[must_use]
pub fn view_proj_gl(yaw: f64, w_px: f64, h_px: f64) -> [f64; 16] {
    let aspect = if h_px <= 0.0 { 1.0 } else { w_px / h_px };
    multiply(&perspective_no(FOVY, aspect, NEAR, FAR), &view(yaw))
}

/// The render uniform: `Z01 · P · V`, f64-composed, cast to f32 last (per-instance model
/// matrices multiply in the shader).
#[must_use]
pub fn view_proj_wgpu(yaw: f64, w_px: f64, h_px: f64) -> [f32; 16] {
    let m = multiply(&Z01, &view_proj_gl(yaw, w_px, h_px));
    core::array::from_fn(|i| m[i] as f32)
}

// ── picking ──────────────────────────────────────────────────────────────────

/// Ray/unit-box slab test in the instance's local space. Unit primitives are centered at
/// the origin with extent ±0.5 (the cylinder picks as its bounding box — schematic doll,
/// box-accuracy is the product bar). Returns the entry parameter `t` when hit.
fn ray_unit_box(origin: [f64; 3], dir: [f64; 3]) -> Option<f64> {
    let mut t_min = f64::NEG_INFINITY;
    let mut t_max = f64::INFINITY;
    for a in 0..3 {
        if dir[a].abs() < 1e-12 {
            if origin[a].abs() > 0.5 {
                return None;
            }
            continue;
        }
        let inv = 1.0 / dir[a];
        let t0 = (-0.5 - origin[a]) * inv;
        let t1 = (0.5 - origin[a]) * inv;
        let (lo, hi) = if t0 < t1 { (t0, t1) } else { (t1, t0) };
        t_min = t_min.max(lo);
        t_max = t_max.min(hi);
        if t_min > t_max {
            return None;
        }
    }
    if t_max < 0.0 {
        None
    } else {
        Some(t_min.max(0.0))
    }
}

/// Pick the nearest clickable region under a device pixel, or -1. Pure — callable from
/// vitest through the wasm shim without a GPU.
#[must_use]
pub fn pick(yaw: f64, w_px: f64, h_px: f64, x_px: f64, y_px: f64) -> i32 {
    if w_px <= 0.0 || h_px <= 0.0 {
        return -1;
    }
    let Some(inv_vp) = invert(&view_proj_gl(yaw, w_px, h_px)) else {
        return -1;
    };
    let ndc_x = (x_px / w_px) * 2.0 - 1.0;
    let ndc_y = 1.0 - (y_px / h_px) * 2.0;
    let near = transform_vector(&inv_vp, [ndc_x, ndc_y, -1.0, 1.0]);
    let far = transform_vector(&inv_vp, [ndc_x, ndc_y, 1.0, 1.0]);
    let origin = [near[0], near[1], near[2]];
    let dir = [far[0] - near[0], far[1] - near[1], far[2] - near[2]];

    let mut best: Option<(f64, i32)> = None;
    for inst in instances() {
        if inst.region < 0 {
            continue;
        }
        let Some(inv_model) = invert(&inst.model) else {
            continue;
        };
        let o = transform_vector(&inv_model, [origin[0], origin[1], origin[2], 1.0]);
        let d = {
            // Direction transforms without translation: w = 0 (transform_vector divides by
            // w, so apply the linear part manually).
            let m = &inv_model;
            [
                m[0] * dir[0] + m[4] * dir[1] + m[8] * dir[2],
                m[1] * dir[0] + m[5] * dir[1] + m[9] * dir[2],
                m[2] * dir[0] + m[6] * dir[1] + m[10] * dir[2],
            ]
        };
        if let Some(t) = ray_unit_box([o[0], o[1], o[2]], d)
            && best.is_none_or(|(bt, _)| t < bt)
        {
            best = Some((t, inst.region));
        }
    }
    best.map_or(-1, |(_, r)| r)
}

// ── meshes ───────────────────────────────────────────────────────────────────

/// Unit cube (extent ±0.5), per-face normals: 24 vertices × 6 floats (pos+normal
/// interleaved), 36 indices.
#[must_use]
pub fn mesh_cube() -> (Vec<f32>, Vec<u16>) {
    // (normal, four corners CCW seen from outside)
    const FACES: [([f32; 3], [[f32; 3]; 4]); 6] = [
        (
            [0.0, 0.0, 1.0],
            [
                [-0.5, -0.5, 0.5],
                [0.5, -0.5, 0.5],
                [0.5, 0.5, 0.5],
                [-0.5, 0.5, 0.5],
            ],
        ),
        (
            [0.0, 0.0, -1.0],
            [
                [0.5, -0.5, -0.5],
                [-0.5, -0.5, -0.5],
                [-0.5, 0.5, -0.5],
                [0.5, 0.5, -0.5],
            ],
        ),
        (
            [1.0, 0.0, 0.0],
            [
                [0.5, -0.5, 0.5],
                [0.5, -0.5, -0.5],
                [0.5, 0.5, -0.5],
                [0.5, 0.5, 0.5],
            ],
        ),
        (
            [-1.0, 0.0, 0.0],
            [
                [-0.5, -0.5, -0.5],
                [-0.5, -0.5, 0.5],
                [-0.5, 0.5, 0.5],
                [-0.5, 0.5, -0.5],
            ],
        ),
        (
            [0.0, 1.0, 0.0],
            [
                [-0.5, 0.5, 0.5],
                [0.5, 0.5, 0.5],
                [0.5, 0.5, -0.5],
                [-0.5, 0.5, -0.5],
            ],
        ),
        (
            [0.0, -1.0, 0.0],
            [
                [-0.5, -0.5, -0.5],
                [0.5, -0.5, -0.5],
                [0.5, -0.5, 0.5],
                [-0.5, -0.5, 0.5],
            ],
        ),
    ];
    let mut verts = Vec::with_capacity(24 * 6);
    let mut idx = Vec::with_capacity(36);
    for (f, (n, corners)) in FACES.iter().enumerate() {
        let base = u16::try_from(f * 4).expect("cube base fits u16");
        for c in corners {
            verts.extend_from_slice(c);
            verts.extend_from_slice(n);
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (verts, idx)
}

/// Unit cylinder (axis Y, radius 0.5, height 1): `segments` side quads with radial normals
/// plus two cap fans with axial normals. Vertices: `4·s` side + `2·(s+1)` caps; indices:
///   `6·s` side + `2·3·s` caps.
#[must_use]
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
pub fn mesh_cylinder(segments: usize) -> (Vec<f32>, Vec<u16>) {
    let s = segments.max(3);
    let mut verts: Vec<f32> = Vec::new();
    let mut idx: Vec<u16> = Vec::new();
    let ring = |i: usize| {
        let a = (i as f64) / (s as f64) * core::f64::consts::TAU;
        (0.5 * a.cos(), 0.5 * a.sin())
    };
    // Sides: per-segment quad with the segment-start radial normal (flat shading is fine
    // for a schematic doll).
    for i in 0..s {
        let (x0, z0) = ring(i);
        let (x1, z1) = ring(i + 1);
        let n = {
            let (nx, nz) = ((x0 + x1) as f32, (z0 + z1) as f32);
            let len = (nx * nx + nz * nz).sqrt().max(1e-6);
            [nx / len, 0.0, nz / len]
        };
        let base = u16::try_from(verts.len() / 6).expect("cylinder verts fit u16");
        for (x, y, z) in [(x0, -0.5, z0), (x1, -0.5, z1), (x1, 0.5, z1), (x0, 0.5, z0)] {
            verts.extend_from_slice(&[x as f32, y as f32, z as f32]);
            verts.extend_from_slice(&n);
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    // Caps: center + ring fan.
    for (y, ny) in [(0.5_f64, 1.0_f32), (-0.5, -1.0)] {
        let center = u16::try_from(verts.len() / 6).expect("cap center fits u16");
        verts.extend_from_slice(&[0.0, y as f32, 0.0, 0.0, ny, 0.0]);
        for i in 0..s {
            let (x, z) = ring(i);
            verts.extend_from_slice(&[x as f32, y as f32, z as f32, 0.0, ny, 0.0]);
        }
        for i in 0..s {
            let a = center + 1 + u16::try_from(i).expect("cap idx");
            let b = center + 1 + u16::try_from((i + 1) % s).expect("cap idx");
            if ny > 0.0 {
                idx.extend_from_slice(&[center, a, b]);
            } else {
                idx.extend_from_slice(&[center, b, a]);
            }
        }
    }
    (verts, idx)
}

// ── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_keys_count_and_uniqueness() {
        assert_eq!(REGION_KEYS.len(), 14);
        let mut sorted = REGION_KEYS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 14, "duplicate region key");
    }

    #[test]
    fn every_region_has_at_least_one_instance() {
        let inst = instances();
        for (i, key) in REGION_KEYS.iter().enumerate() {
            let n = inst
                .iter()
                .filter(|d| d.region == i32::try_from(i).unwrap())
                .count();
            assert!(n >= 1, "region {key} has no instances");
        }
        assert!(inst.iter().any(|d| d.region == DECOR), "decor body missing");
    }

    #[test]
    fn mesh_counts_exact() {
        let (cv, ci) = mesh_cube();
        assert_eq!(cv.len(), 24 * 6);
        assert_eq!(ci.len(), 36);
        let (yv, yi) = mesh_cylinder(16);
        assert_eq!(yv.len(), (4 * 16 + 2 * 17) * 6);
        assert_eq!(yi.len(), 6 * 16 + 2 * 3 * 16);
    }

    #[test]
    fn perspective_golden() {
        // gl-matrix perspectiveNO(fovy=0.6109, aspect=1.5, near=0.1, far=100) — the exact
        // expression tree recomputed here; spot values guard against transposition.
        let m = perspective_no(0.6109, 1.5, 0.1, 100.0);
        let f = 1.0 / (0.6109_f64 / 2.0).tan();
        assert_eq!(m[0], f / 1.5);
        assert_eq!(m[5], f);
        assert_eq!(m[11], -1.0);
        let nf = 1.0 / (0.1 - 100.0);
        assert_eq!(m[10], (100.0 + 0.1) * nf);
        assert_eq!(m[14], 2.0 * 100.0 * 0.1 * nf);
        assert_eq!(m[15], 0.0);
        // Infinite far branch.
        let inf = perspective_no(0.6109, 1.5, 0.1, f64::INFINITY);
        assert_eq!(inf[10], -1.0);
        assert_eq!(inf[14], -0.2);
    }

    #[test]
    fn pick_goldens_center_regions() {
        // Front view (yaw 0), 800×600 device px. Screen center height ≈ torso: the rifle
        // hangs across the chest there — expect the receiver, not the jacket.
        let w = 800.0;
        let h = 600.0;
        let center = pick(0.0, w, h, 400.0, 300.0);
        assert_eq!(
            REGION_KEYS[usize::try_from(center).expect("hit")],
            "primary",
            "screen center should hit the rifle receiver"
        );
        // Well above the soldier: miss.
        assert_eq!(pick(0.0, w, h, 400.0, 10.0), -1);
        // Far off to the side: miss.
        assert_eq!(pick(0.0, w, h, 20.0, 300.0), -1);
    }

    #[test]
    fn pick_yaw_symmetry_hits_backpack_from_behind() {
        // From behind (yaw = π) the torso center is covered by the backpack.
        let hit = pick(core::f64::consts::PI, 800.0, 600.0, 400.0, 280.0);
        assert!(hit >= 0, "back view center must hit something");
        let key = REGION_KEYS[usize::try_from(hit).expect("hit")];
        assert!(
            key == "backpack" || key == "armoredVest" || key == "launcher",
            "back view center hit {key}, expected back-mounted gear"
        );
    }

    #[test]
    fn state_colors_distinct() {
        let e = state_color(STATE_EMPTY);
        let q = state_color(STATE_EQUIPPED);
        let a = state_color(STATE_ACTIVE);
        assert_ne!(e, q);
        assert_ne!(q, a);
        assert_ne!(e, a);
        for c in [e, q, a] {
            assert_eq!(c[3], 1.0, "opaque pipeline — no alpha");
        }
    }
}
