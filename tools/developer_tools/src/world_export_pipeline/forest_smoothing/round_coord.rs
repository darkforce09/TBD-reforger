use super::*;

pub(super) fn round_coord(v: f64) -> f64 {
    let scale = 10f64.powi(COORD_DECIMALS);
    (v * scale).round() / scale
}

/// Signed shoelace area of a ring given by its **distinct** vertices (closure implied).
#[must_use]
pub fn signed_area(v: &[(f64, f64)]) -> f64 {
    let n = v.len();
    let mut s = 0.0;
    for i in 0..n {
        let j = if i + 1 == n { 0 } else { i + 1 };
        s += v[i].0 * v[j].1 - v[j].0 * v[i].1;
    }
    s / 2.0
}

/// Unit left normal of a direction; `None` for a zero-length edge.
pub(super) fn left_normal(dx: f64, dy: f64) -> Option<(f64, f64)> {
    let len = dx.hypot(dy);
    if len < EPS {
        return None;
    }
    Some((-dy / len, dx / len))
}

/// Unit vertex normals (the angle bisector of the two adjacent edge normals, on the ring's own
/// "left" side). Falls back to a single edge normal at a spike, and to (0,0) only when both edges
/// are degenerate — an offset of zero there is the safe answer.
pub(crate) fn vertex_normals(v: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let n = v.len();
    (0..n)
        .map(|i| {
            let p = v[(i + n - 1) % n];
            let c = v[i];
            let q = v[(i + 1) % n];
            let a = left_normal(c.0 - p.0, c.1 - p.1);
            let b = left_normal(q.0 - c.0, q.1 - c.1);
            match (a, b) {
                (Some(a), Some(b)) => {
                    let (sx, sy) = (a.0 + b.0, a.1 + b.1);
                    let len = sx.hypot(sy);
                    if len < EPS { a } else { (sx / len, sy / len) }
                }
                (Some(a), None) => a,
                (None, Some(b)) => b,
                (None, None) => (0.0, 0.0),
            }
        })
        .collect()
}

/// Which vertices the 8 m canopy holds sharp. Decided on the INPUT ring — after one Chaikin pass
/// the vertices no longer sit on the 32 m lattice and "is this lattice corner real" is no longer
/// the question being asked.
///
/// `sign` is the ring's orientation (`signed_area().signum()`), so the rule reads the same for an
/// outer ring and for a hole: `cross > 0` is a convex corner of the *material*, and `+bisector` is
/// into the material either way.
pub(crate) fn corner_pins(v: &[(f64, f64)], sign: f64, canopy: CanopyMass<'_>) -> Vec<bool> {
    let n = v.len();
    (0..n)
        .map(|i| {
            let p = v[(i + n - 1) % n];
            let c = v[i];
            let q = v[(i + 1) % n];
            let (ax, ay) = (c.0 - p.0, c.1 - p.1);
            let (bx, by) = (q.0 - c.0, q.1 - c.1);
            // Two DIFFERENT questions, and conflating them inverts every hole:
            //   `turn`  — which SIDE of the ring the cut's wedge lies on, relative to the ring's
            //             own enclosed region. Orientation-normalised, so it reads the same for an
            //             outer ring and a hole.
            //   `raw`   — whether the cut takes MATERIAL away or gives it back. A hole encloses a
            //             clearing, so shrinking a hole's enclosure *adds* forest; the raw turn
            //             already carries that flip and must not be normalised out.
            let raw = ax * by - ay * bx;
            let turn = raw * sign;
            let (Some(na), Some(nb)) = (left_normal(ax, ay), left_normal(bx, by)) else {
                return false;
            };
            let (sx, sy) = ((na.0 + nb.0) * sign, (na.1 + nb.1) * sign);
            let len = sx.hypot(sy);
            if turn.abs() < EPS || len < EPS {
                // Collinear: there is no corner here to preserve.
                return false;
            }
            // Unit vector into the ring's own enclosed region (the material for an outer ring,
            // the clearing for a hole).
            let (ix, iy) = (sx / len, sy / len);
            // The wedge the cut moves the boundary across: inside the enclosure at a ring-convex
            // vertex, outside it at a ring-reflex one.
            let probe = if turn > 0.0 {
                (c.0 + CORNER_PROBE_M * ix, c.1 + CORNER_PROBE_M * iy)
            } else {
                (c.0 - CORNER_PROBE_M * ix, c.1 - CORNER_PROBE_M * iy)
            };
            let mass = canopy(probe.0, probe.1);
            if raw > 0.0 {
                // The cut EATS forest here. Keep the corner only if the wedge is solid canopy.
                mass >= PIN_SOLID_MASS
            } else {
                // The cut GROWS forest into the wedge. Keep the notch only if the wedge is bare.
                mass <= PIN_CLEAR_MASS
            }
        })
        .collect()
}

pub(super) fn lerp(a: (f64, f64), b: (f64, f64), t: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// One corner-preserving Chaikin pass over a closed ring of distinct vertices. A pinned vertex is
/// emitted as itself and neither of the two points that would have cut it is produced, so the
/// pin survives every subsequent iteration.
pub(crate) fn chaikin_once(v: &[(f64, f64)], pin: &[bool]) -> (Vec<(f64, f64)>, Vec<bool>) {
    let n = v.len();
    let mut out = Vec::with_capacity(n * 2);
    let mut out_pin = Vec::with_capacity(n * 2);
    for i in 0..n {
        let a = v[i];
        let j = if i + 1 == n { 0 } else { i + 1 };
        let b = v[j];
        if pin[i] {
            out.push(a);
            out_pin.push(true);
        } else {
            out.push(lerp(a, b, CHAIKIN_CUT));
            out_pin.push(false);
        }
        if pin[j] {
            continue;
        }
        out.push(lerp(a, b, 1.0 - CHAIKIN_CUT));
        out_pin.push(false);
    }
    (out, out_pin)
}

/// Signed area of `v` offset by `d` along `u`, as an exact quadratic `A + B·d + C·d²`.
///
/// Closed form rather than a bisection: shoelace is bilinear in the vertices, so substituting
/// `P_i + d·u_i` gives a quadratic in `d` whose coefficients fall straight out. Deterministic in
/// one step, with no iteration count to tune and no bracketing to get wrong.
pub(super) fn offset_area_coefficients(v: &[(f64, f64)], u: &[(f64, f64)]) -> (f64, f64, f64) {
    let n = v.len();
    let (mut a, mut b, mut c) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let j = if i + 1 == n { 0 } else { i + 1 };
        a += v[i].0 * v[j].1 - v[j].0 * v[i].1;
        b += v[i].0 * u[j].1 + u[i].0 * v[j].1 - v[j].0 * u[i].1 - u[j].0 * v[i].1;
        c += u[i].0 * u[j].1 - u[j].0 * u[i].1;
    }
    (a / 2.0, b / 2.0, c / 2.0)
}

/// Mean edge length of a closed ring given by its distinct vertices.
pub(crate) fn mean_edge(v: &[(f64, f64)]) -> f64 {
    let n = v.len();
    if n == 0 {
        return 0.0;
    }
    let total: f64 = (0..n)
        .map(|i| {
            let j = if i + 1 == n { 0 } else { i + 1 };
            (v[j].0 - v[i].0).hypot(v[j].1 - v[i].1)
        })
        .sum();
    total / n as f64
}

/// The offset `d` that restores `target` signed area, railed by [`MAX_AREA_OFFSET_EDGES`].
/// Returns `(d, capped)`.
pub(crate) fn solve_offset(v: &[(f64, f64)], u: &[(f64, f64)], target: f64) -> (f64, bool) {
    let (a, b, c) = offset_area_coefficients(v, u);
    let k = a - target;
    let raw = if c.abs() < EPS {
        // Degenerate to a straight line: a ring so smooth the quadratic term vanishes.
        if b.abs() < EPS {
            return (0.0, false);
        }
        -k / b
    } else {
        let disc = b * b - 4.0 * c * k;
        if disc < 0.0 {
            // The parabola never reaches the target; its vertex is the closest we can get.
            -b / (2.0 * c)
        } else {
            let r = disc.sqrt();
            let r1 = (-b + r) / (2.0 * c);
            let r2 = (-b - r) / (2.0 * c);
            // The root nearest zero: the smallest boundary displacement that does the job.
            if r1.abs() <= r2.abs() { r1 } else { r2 }
        }
    };
    if !raw.is_finite() {
        return (0.0, false);
    }
    let rail = MAX_AREA_OFFSET_EDGES * mean_edge(v);
    let clamped = raw.clamp(-rail, rail);
    (clamped, (clamped - raw).abs() > EPS)
}

/// Drop consecutive duplicate vertices left by coordinate rounding. Cannot fire on real rings
/// (the shortest post-Chaikin edge on a 32 m lattice is metres, not centimetres) but a ring that
/// collapsed here would violate the ≥4-entry closed-ring schema, so it is checked, not assumed.
pub(super) fn dedupe(v: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::with_capacity(v.len());
    for p in v {
        if out.last().is_some_and(|l| *l == p) {
            continue;
        }
        out.push(p);
    }
    while out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}

/// Smooth one **closed** ring (`first == last`, as `forest::canonicalize_ring` emits them):
/// [`chaikin`] behind the LOCKED sub-[`MIN_SMOOTH_VERTICES`] carve-out. This is the emit's entry
/// point; `chaikin` is the geometry underneath it.
#[must_use]
pub fn smooth_ring(
    ring: &[(f64, f64)],
    canopy: Option<CanopyMass<'_>>,
) -> (Vec<(f64, f64)>, RingReport) {
    let mut v: Vec<(f64, f64)> = ring.to_vec();
    while v.len() > 1 && v.first() == v.last() {
        v.pop();
    }
    if v.len() < MIN_SMOOTH_VERTICES {
        let area = signed_area(&v);
        return (
            ring.to_vec(),
            RingReport {
                verts_in: v.len(),
                verts_out: v.len(),
                area_in: area,
                area_out: area,
                skipped_small: true,
                ..RingReport::default()
            },
        );
    }
    chaikin(ring, CHAIKIN_ITERATIONS, canopy)
}

/// Corner-preserving Chaikin over a closed ring, with the area restored — the plan's
/// `chaikin(ring, iters, keep_corners)`.
///
/// No size carve-out: this rounds whatever it is handed, including a bare four-vertex square (see
/// `a_square_ring_rounds`). [`smooth_ring`] is the one that applies [`MIN_SMOOTH_VERTICES`],
/// because that rule is about which rings the *emit* should leave alone, not about what corner
/// cutting can do.
///
/// Returns a closed ring. A ring that would collapse under the rounding comes back byte-for-byte
/// as it went in.
#[must_use]
pub fn chaikin(
    ring: &[(f64, f64)],
    iterations: usize,
    canopy: Option<CanopyMass<'_>>,
) -> (Vec<(f64, f64)>, RingReport) {
    let mut v: Vec<(f64, f64)> = ring.to_vec();
    while v.len() > 1 && v.first() == v.last() {
        v.pop();
    }
    let mut report = RingReport {
        verts_in: v.len(),
        verts_out: v.len(),
        area_in: signed_area(&v),
        ..RingReport::default()
    };
    report.area_out = report.area_in;
    if v.len() < 3 {
        report.skipped_small = true;
        return (ring.to_vec(), report);
    }

    let sign = if report.area_in >= 0.0 { 1.0 } else { -1.0 };
    let mut pin = match canopy {
        Some(c) => corner_pins(&v, sign, c),
        None => vec![false; v.len()],
    };
    report.pinned = pin.iter().filter(|p| **p).count();

    for _ in 0..iterations {
        let (nv, np) = chaikin_once(&v, &pin);
        v = nv;
        pin = np;
    }

    let u = vertex_normals(&v);
    let (d, capped) = solve_offset(&v, &u, report.area_in);
    report.offset_m = d;
    report.offset_capped = capped;
    let smoothed: Vec<(f64, f64)> = v
        .iter()
        .zip(&u)
        .map(|(p, n)| (round_coord(p.0 + d * n.0), round_coord(p.1 + d * n.1)))
        .collect();

    let smoothed = dedupe(smoothed);
    if smoothed.len() < 3 {
        // Would not be a valid ring. Emit the input rather than a degenerate polygon.
        report.skipped_small = true;
        report.offset_m = 0.0;
        report.offset_capped = false;
        report.pinned = 0;
        return (ring.to_vec(), report);
    }
    report.verts_out = smoothed.len();
    report.area_out = signed_area(&smoothed);

    let mut closed = smoothed;
    closed.push(closed[0]);
    (closed, report)
}

/// Read one `[[x, y], …]` JSON ring. `None` if any vertex is not a finite `[number, number]`.
pub(crate) fn ring_from_json(ring: &Value) -> Option<Vec<(f64, f64)>> {
    let arr = ring.as_array()?;
    arr.iter()
        .map(|p| {
            let p = p.as_array()?;
            let x = p.first()?.as_f64()?;
            let y = p.get(1)?.as_f64()?;
            (x.is_finite() && y.is_finite()).then_some((x, y))
        })
        .collect()
}

pub(super) fn ring_to_json(ring: &[(f64, f64)]) -> Value {
    Value::Array(
        ring.iter()
            .map(|(x, y)| Value::Array(vec![js_num(*x), js_num(*y)]))
            .collect(),
    )
}

/// Smooth every ring of every region **in place**, returning one report per region.
///
/// A region whose `polygon` is missing or unreadable is left exactly as it was and still gets a
/// report (with zero rings), so a malformed row cannot vanish from the emit's own accounting.
pub fn smooth_regions(regions: &mut [Value], canopy: Option<CanopyMass<'_>>) -> Vec<RegionReport> {
    regions
        .iter_mut()
        .map(|region| {
            let id = region["id"].as_str().unwrap_or("?").to_string();
            let mut out = RegionReport {
                id,
                ..RegionReport::default()
            };
            let Some(rings) = region.get("polygon").and_then(Value::as_array).cloned() else {
                return out;
            };
            let mut smoothed_rings: Vec<Value> = Vec::with_capacity(rings.len());
            for ring_json in &rings {
                let Some(ring) = ring_from_json(ring_json) else {
                    smoothed_rings.push(ring_json.clone());
                    continue;
                };
                let (smoothed, r) = smooth_ring(&ring, canopy);
                out.rings += 1;
                out.verts_in += r.verts_in;
                out.verts_out += r.verts_out;
                out.pinned += r.pinned;
                out.area_in += r.area_in;
                out.area_out += r.area_out;
                out.max_offset_m = out.max_offset_m.max(r.offset_m.abs());
                out.offset_capped += usize::from(r.offset_capped);
                smoothed_rings.push(ring_to_json(&smoothed));
            }
            region["polygon"] = Value::Array(smoothed_rings);
            out
        })
        .collect()
}

/// The emit's per-region log: vertex count and area drift for every region, then the roll-up.
/// Anything over [`MAX_AREA_DRIFT`] or a capped offset is called out by name — a smoothing pass
/// that quietly moved a forest is the failure mode this whole module has to not have.
pub fn log_reports(terrain: &str, reports: &[RegionReport]) {
    let mut worst = 0.0f64;
    let (mut vin, mut vout, mut pinned, mut capped) = (0usize, 0usize, 0usize, 0usize);
    for r in reports {
        let drift = r.area_drift();
        worst = worst.max(drift);
        vin += r.verts_in;
        vout += r.verts_out;
        pinned += r.pinned;
        capped += r.offset_capped;
        eprintln!(
            "[forest-smooth] {terrain}: {} {} rings, {} -> {} verts ({} pinned), \
             area {:.1} -> {:.1} m² (drift {:.4}%), max offset {:.3} m{}",
            r.id,
            r.rings,
            r.verts_in,
            r.verts_out,
            r.pinned,
            r.area_in.abs(),
            r.area_out.abs(),
            drift * 100.0,
            r.max_offset_m,
            if drift > MAX_AREA_DRIFT {
                "  *** OVER THE 3% ACCEPTANCE BOUND ***"
            } else {
                ""
            }
        );
    }
    eprintln!(
        "[forest-smooth] {terrain}: {} regions, {vin} -> {vout} verts, {pinned} pinned, \
         worst area drift {:.4}% (bound {:.1}%), {capped} offset(s) capped",
        reports.len(),
        worst * 100.0,
        MAX_AREA_DRIFT * 100.0
    );
}
