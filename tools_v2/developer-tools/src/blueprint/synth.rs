//! Synthetic dump generators for the test suite: analytic buildings marched exactly like the
//! Enfusion dumper (entry face per solid front, both directions, normalized coordinates), so the
//! pipeline is exercised end-to-end without a Workbench in the room.

use super::march::{self, CELL, DumpIdent};
use super::types::VoxelDump;

/// Analytic solid in LOCAL coordinates (pre-pad).
enum Solid {
    Box3 {
        min: [f64; 3],
        max: [f64; 3],
    },
    /// Vertical wall slab at x ∈ [x0,x1] whose top follows the gable profile
    /// h(z) = ridge - |z - zc| * slope, from y0 up.
    GableWall {
        x0: f64,
        x1: f64,
        y0: f64,
        zc: f64,
        ridge: f64,
        slope: f64,
        z0: f64,
        z1: f64,
    },
    /// Sloped roof plane pair (both pitches), vertical thickness `thick`, over z ∈ [z0,z1].
    RoofPlane {
        x0: f64,
        x1: f64,
        zc: f64,
        ridge: f64,
        slope: f64,
        thick: f64,
        z0: f64,
        z1: f64,
    },
}

impl Solid {
    fn gable_h(zc: f64, ridge: f64, slope: f64, z: f64) -> f64 {
        ridge - (z - zc).abs() * slope
    }

    /// Solid intervals crossed by an axis line. `axis`: 0=x line at (fy=a, fz=b),
    /// 1=y line at (fx=a, fz=b), 2=z line at (fx=a, fy=b).
    fn intervals(&self, axis: usize, a: f64, b: f64) -> Vec<(f64, f64)> {
        match *self {
            Solid::Box3 { min, max } => {
                let (lo, hi, in1, in2) = match axis {
                    0 => (min[0], max[0], (a, min[1], max[1]), (b, min[2], max[2])),
                    1 => (min[1], max[1], (a, min[0], max[0]), (b, min[2], max[2])),
                    _ => (min[2], max[2], (a, min[0], max[0]), (b, min[1], max[1])),
                };
                let inside = |v: (f64, f64, f64)| v.0 >= v.1 && v.0 < v.2;
                if inside(in1) && inside(in2) {
                    vec![(lo, hi)]
                } else {
                    vec![]
                }
            }
            Solid::GableWall {
                x0,
                x1,
                y0,
                zc,
                ridge,
                slope,
                z0,
                z1,
            } => match axis {
                0 => {
                    let (fy, fz) = (a, b);
                    let h = Self::gable_h(zc, ridge, slope, fz);
                    if fz >= z0 && fz < z1 && fy >= y0 && fy < h {
                        vec![(x0, x1)]
                    } else {
                        vec![]
                    }
                }
                1 => {
                    let (fx, fz) = (a, b);
                    let h = Self::gable_h(zc, ridge, slope, fz);
                    if fx >= x0 && fx < x1 && fz >= z0 && fz < z1 && h > y0 {
                        vec![(y0, h)]
                    } else {
                        vec![]
                    }
                }
                _ => {
                    let (fx, fy) = (a, b);
                    if fx < x0 || fx >= x1 || fy < y0 || fy >= ridge {
                        return vec![];
                    }
                    let o = (ridge - fy) / slope;
                    vec![((zc - o).max(z0), (zc + o).min(z1))]
                }
            },
            Solid::RoofPlane {
                x0,
                x1,
                zc,
                ridge,
                slope,
                thick,
                z0,
                z1,
            } => match axis {
                0 => {
                    let (fy, fz) = (a, b);
                    let h = Self::gable_h(zc, ridge, slope, fz);
                    if fz >= z0 && fz < z1 && fy >= h - thick && fy < h {
                        vec![(x0, x1)]
                    } else {
                        vec![]
                    }
                }
                1 => {
                    let (fx, fz) = (a, b);
                    let h = Self::gable_h(zc, ridge, slope, fz);
                    if fx >= x0 && fx < x1 && fz >= z0 && fz < z1 {
                        vec![(h - thick, h)]
                    } else {
                        vec![]
                    }
                }
                _ => {
                    let (fx, fy) = (a, b);
                    if fx < x0 || fx >= x1 || fy >= ridge {
                        return vec![];
                    }
                    // h(z) ∈ [fy, fy + thick] on both pitches.
                    let (o1, o2) = ((ridge - fy - thick) / slope, (ridge - fy) / slope);
                    let (o1, o2) = (o1.max(0.0), o2.max(0.0));
                    let mut out = Vec::new();
                    let left = ((zc - o2).max(z0), (zc - o1).min(z1));
                    let right = ((zc + o1).max(z0), (zc + o2).min(z1));
                    if left.1 > left.0 {
                        out.push(left);
                    }
                    if right.1 > right.0 && (out.is_empty() || right.0 > out[0].1) {
                        out.push(right);
                    }
                    out
                }
            },
        }
    }
}

fn merged_intervals(solids: &[Solid], axis: usize, a: f64, b: f64) -> Vec<(f64, f64)> {
    let mut ivs: Vec<(f64, f64)> = solids
        .iter()
        .flat_map(|s| s.intervals(axis, a, b))
        .collect();
    ivs.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut out: Vec<(f64, f64)> = Vec::new();
    for iv in ivs {
        match out.last_mut() {
            Some(last) if iv.0 <= last.1 + 0.02 => last.1 = last.1.max(iv.1),
            _ => out.push(iv),
        }
    }
    out
}

/// March all six directions over the solids via the shared [`march`] skeleton (which owns the
/// pad/origin/dims math and r2 normalization): forward entries are interval starts ascending,
/// backward entries are interval ends in reverse — exactly the closing faces a −axis march sees.
fn generate(solids: Vec<Solid>, bbox_min: [f64; 3], bbox_max: [f64; 3]) -> VoxelDump {
    march::generate_dump(
        DumpIdent {
            slug: "synth".into(),
            resource: "synth://".into(),
        },
        bbox_min,
        bbox_max,
        |axis, a, b| {
            let ivs = merged_intervals(&solids, axis, a, b);
            let fwd: Vec<f64> = ivs.iter().map(|iv| iv.0).collect();
            let mut bwd: Vec<f64> = ivs.iter().map(|iv| iv.1).collect();
            bwd.reverse();
            (fwd, bwd)
        },
    )
}

fn shell(w: f64, d: f64, h: f64, t: f64) -> Vec<Solid> {
    vec![
        Solid::Box3 {
            min: [0.0, -0.12, 0.0],
            max: [w, 0.0, d],
        }, // floor plate
        Solid::Box3 {
            min: [0.0, h, 0.0],
            max: [w, h + 0.12, d],
        }, // flat roof
        Solid::Box3 {
            min: [0.0, 0.0, 0.0],
            max: [t, h, d],
        }, // west
        Solid::Box3 {
            min: [w - t, 0.0, 0.0],
            max: [w, h, d],
        }, // east
        Solid::Box3 {
            min: [t, 0.0, 0.0],
            max: [w - t, h, t],
        }, // south
        Solid::Box3 {
            min: [t, 0.0, d - t],
            max: [w - t, h, d],
        }, // north
    ]
}

pub fn box_room(w: f64, d: f64, h: f64, t: f64) -> VoxelDump {
    generate(shell(w, d, h, t), [0.0, -0.12, 0.0], [w, h + 0.12, d])
}

/// Box room with a doorway (width `door_w`, lintel at `door_h`) cut into the south wall.
pub fn box_with_door(w: f64, d: f64, h: f64, t: f64, door_x: f64, door_w: f64) -> VoxelDump {
    let door_h = 2.0;
    let mut solids = vec![
        Solid::Box3 {
            min: [0.0, -0.12, 0.0],
            max: [w, 0.0, d],
        },
        Solid::Box3 {
            min: [0.0, h, 0.0],
            max: [w, h + 0.12, d],
        },
        Solid::Box3 {
            min: [0.0, 0.0, 0.0],
            max: [t, h, d],
        },
        Solid::Box3 {
            min: [w - t, 0.0, 0.0],
            max: [w, h, d],
        },
        Solid::Box3 {
            min: [t, 0.0, d - t],
            max: [w - t, h, d],
        },
    ];
    solids.push(Solid::Box3 {
        min: [t, 0.0, 0.0],
        max: [door_x, h, t],
    });
    solids.push(Solid::Box3 {
        min: [door_x + door_w, 0.0, 0.0],
        max: [w - t, h, t],
    });
    solids.push(Solid::Box3 {
        min: [door_x, door_h, 0.0],
        max: [door_x + door_w, h, t],
    });
    generate(solids, [0.0, -0.12, 0.0], [w, h + 0.12, d])
}

/// The gable-house solid set shared by [`gable_box`] and [`gable_mezzanine`]: floor plate,
/// four eave-height walls, west/east gable-end triangles, and the sloped roof planes.
fn gable_solids(w: f64, d: f64, eave: f64, ridge: f64, t: f64) -> Vec<Solid> {
    let zc = d / 2.0;
    let slope = (ridge - eave) / zc;
    vec![
        Solid::Box3 {
            min: [0.0, -0.12, 0.0],
            max: [w, 0.0, d],
        },
        Solid::Box3 {
            min: [0.0, 0.0, 0.0],
            max: [t, eave, d],
        },
        Solid::Box3 {
            min: [w - t, 0.0, 0.0],
            max: [w, eave, d],
        },
        Solid::Box3 {
            min: [t, 0.0, 0.0],
            max: [w - t, eave, t],
        },
        Solid::Box3 {
            min: [t, 0.0, d - t],
            max: [w - t, eave, d],
        },
        Solid::GableWall {
            x0: 0.0,
            x1: t,
            y0: eave,
            zc,
            ridge,
            slope,
            z0: 0.0,
            z1: d,
        },
        Solid::GableWall {
            x0: w - t,
            x1: w,
            y0: eave,
            zc,
            ridge,
            slope,
            z0: 0.0,
            z1: d,
        },
        Solid::RoofPlane {
            x0: 0.0,
            x1: w,
            zc,
            ridge,
            slope,
            thick: 0.15,
            z0: 0.0,
            z1: d,
        },
    ]
}

/// One-story box with a gable roof: ridge along x at z = d/2, gable-end triangles on the west
/// and east walls, sloped planes elsewhere.
pub fn gable_box(w: f64, d: f64, eave: f64, ridge: f64, t: f64) -> VoxelDump {
    let solids = gable_solids(w, d, eave, ridge, t);
    generate(solids, [0.0, -0.12, 0.0], [w, ridge + 0.1, d])
}

/// Gable house with a PARTIAL mezzanine and a knee wall — the floors-and-walls fixtures:
/// - mezzanine slab at 2.0 m (top face; 1.8 m slab spacing keeps it a distinct floor) over the
///   WEST half only — the east half is open floor-to-roof, so the level-1 plate must trace a
///   partial ring and leave the void void;
/// - a floor-to-2.9 m wall on the open side near the south eave, under the roof plane at
///   ~3.0 m there: in band 1 [2.0, 4.0] it shows in only ~6 of 16 slice rows (old global
///   persistence = dropped) yet fills its ROOF-CLIPPED window — the per-column regression;
/// - ridge 5.2 over the 4.0 band top ⇒ the attic band self-synthesizes (rise 1.2 ≥ 1.0).
pub fn gable_mezzanine() -> VoxelDump {
    let (w, d, eave, ridge, t) = (6.0, 4.0, 2.6, 5.2, 0.15);
    let mut solids = gable_solids(w, d, eave, ridge, t);
    solids.push(Solid::Box3 {
        min: [t, 1.88, t],
        max: [3.0, 2.0, d - t],
    }); // mezzanine slab, west half
    solids.push(Solid::Box3 {
        min: [3.2, 0.0, 0.25],
        max: [5.0, 2.9, 0.4],
    }); // knee wall on the open side, top under the descending roof
    generate(solids, [0.0, -0.12, 0.0], [w, ridge + 0.1, d])
}

/// The grid-vs-segments regression scenario: a two-story box (walls to the flat roof at 5.4 m,
/// second slab at 2.6 m) plus an injected steep roof graze in the second band — a thin solid
/// whose center drifts 0.015 m per 0.1 m slice. Over the 0.35 m between the live pipeline's two
/// probe heights that is ~0.05 m (same 0.1 m cell → survives the AND); over the segments
/// algorithm's ~1.5 m slice window it is ~0.22 m of drift (killed by the 0.08 m stationarity cap).
pub fn steep_graze() -> VoxelDump {
    let (w, d, h, t) = (6.0, 4.0, 5.4, 0.15);
    let mut solids = shell(w, d, h, t);
    solids.push(Solid::Box3 {
        min: [t, 2.48, t],
        max: [w - t, 2.6, d - t],
    }); // 2nd slab
    let mut dump = generate(solids, [0.0, -0.12, 0.0], [w, h + 0.12, d]);

    let m = dump.meta().clone();
    let iy0 = ((2.6 + 0.3 - m.origin[1]) / CELL - 0.5).ceil() as usize;
    let iy1 = ((2.6 + 2.2 - m.origin[1]) / CELL - 0.5).floor() as usize;
    let (iz0, iz1) = (12usize, 30usize);
    for iy in iy0..=iy1 {
        let center = 3.03 + (iy - iy0) as f64 * 0.015;
        let (a, b) = ((center - 0.05_f64).max(0.0), center + 0.05);
        let r2 = |v: f64| (v * 100.0).round() / 100.0;
        for iz in iz0..iz1 {
            let fwd = dump.x_pos.entry((iy, iz)).or_default();
            fwd.push(r2(a));
            fwd.sort_by(f64::total_cmp);
            let bwd = dump.x_neg.entry((iy, iz)).or_default();
            bwd.push(r2(b));
            bwd.sort_by(|x, y| y.total_cmp(x));
        }
    }
    dump
}
