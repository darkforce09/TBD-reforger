//! T-149 — Chaikin smoothing for the Path B forest rings.
//!
//! `forest::trace_rings` walks cell boundaries on the 32 m region lattice, so **every** emitted
//! ring segment is axis-aligned and every turn is exactly ±90°: measured on the committed everon
//! catalogue, 15 528 / 15 528 segments (100.00%). That is the "blocky" this module fixes.
//!
//! Three stages, in order, per ring:
//!
//! 1. **Corner pins** (the corner-preserving option) — decided ONCE on the input ring, where the
//!    8 m density grid is meaningful, then carried through every Chaikin iteration.
//! 2. **Chaikin**, [`CHAIKIN_ITERATIONS`] iterations at cut [`CHAIKIN_CUT`], pinned vertices
//!    passed through uncut.
//! 3. **An area-restoring normal offset** — one scalar per ring, solved in closed form.
//!
//! WHY STAGE 3 EXISTS, and it is not optional: corner cutting is not area-neutral. It removes a
//! wedge at every convex corner and adds one at every concave corner, and on the real everon rings
//! those do **not** cancel — plain 2-iteration Chaikin drifts 11.72% on `forest-everon-036` and
//! puts 22 of 36 regions over the 3% acceptance bound (measured 2026-09-06). The corner-preserving
//! option does not rescue it either: pinning trades convex losses for concave gains and made the
//! worst region *worse* in the same measurement. So the drift is paid back geometrically, by
//! sliding the whole smoothed ring along its own vertex normals by a single distance `d` chosen so
//! the signed area comes back exactly. `d` is a *uniform* boundary displacement — on everon it
//! never exceeds 2.91 m, under a tenth of one 32 m lattice cell — which is why this and not a
//! scale-about-the-centroid: a centroid scale is a similarity, so restoring a 11.7% area loss
//! means a 6% linear inflation, and on a 600 m forest that moves the far edge ~18 m, five times
//! further than the smoothing it is compensating for.
//!
//! WHAT THE 8 m GRID IS FOR. The rings are a 32 m quantisation of a boundary the exporter also
//! holds at 8 m (`density::sample_corners` over the canopy-blurred tree channel). A ±90° turn on
//! the coarse lattice is a real cartographic corner only if the finer field turns there too, and
//! on a rectilinear ring the turn angle alone cannot tell you — every turn is 90°. So the pin
//! rule asks the finer field whether the cut is *justified*, and only overrides the smoother when
//! the evidence is unambiguous ([`PIN_SOLID_MASS`] / [`PIN_CLEAR_MASS`]); the ambiguous band in
//! between is exactly where the 32 m quantisation error lives, and there the ring rounds.
//! Note that the question is about MATERIAL, not about the ring: a hole encloses a clearing, so
//! shrinking a hole's enclosure *adds* forest, and a hole must read the field the other way round
//! from its outer twin (`a_hole_reads_the_canopy_the_other_way_round`).
//!
//! The TBDD format, its header and its writers are untouched — this module only ever *reads*
//! `density::sample_corners`.

use map_engine_core::geometry::forest_mass::CANOPY_MASS_ISO;
use serde_json::Value;

use crate::density;
use crate::forest::js_num;

/// Chaikin corner cut: the two replacement points sit at `t` and `1 - t` along each segment.
pub const CHAIKIN_CUT: f64 = 0.25;

/// Iterations run at the emit (the ticket's "two iterations").
pub const CHAIKIN_ITERATIONS: usize = 2;

/// LOCKED (`docs/specs/ideas/t149_forest_smooth.md`): a ring with fewer *distinct* vertices than
/// this is emitted untouched. A single 32 m cell — a lone clearing inside a forest — traces a
/// 4-vertex ring, and rounding a 32 m square into a lens would lose a sixth of it for no
/// cartographic gain. On everon this carve-out covers 665 rings / 2 660 vertices; those plus the
/// canopy pins are the whole of the residual right-angle count after smoothing.
pub const MIN_SMOOTH_VERTICES: usize = 6;

/// Emitted coordinate precision, in decimal places (centimetres) — the same 2 dp the density
/// module rounds instance positions to. Rounding is the last step, so it perturbs the restored
/// area slightly: measured worst case on everon is 0.0126%, i.e. 238× inside the 3% bound.
pub const COORD_DECIMALS: i32 = 2;

/// Corner probe distance: one density cell out along the corner bisector. Anything shorter lands
/// back on the vertex's own 8 m corner (a diagonal step of `d` moves `d/√2` per axis, and the
/// corner window is ±4 m), so the probe would degrade into "sample the vertex" and tell the
/// smoother nothing it did not already know.
pub const CORNER_PROBE_M: f64 = DENSITY_CELL_M_F;

/// Safety rail on the area-restoring offset, as a multiple of the smoothed ring's **mean edge
/// length** — a rail against pulling a ring through itself, and deliberately not an absolute
/// distance.
///
/// The offset a ring needs is scale-free: `|d| / mean edge` is **0.2258 for a Chaikin-rounded
/// square of any side** (asserted at 32 m, 128 m and 1280 m by
/// `a_square_ring_holds_its_area_under_the_bound_at_every_scale`) and 0.1907 at worst over every
/// everon ring (asserted by `everon_smooths…`). An absolute rail therefore cannot be right for
/// both — an 8 m rail passes the whole everon catalogue (worst offset 2.90 m) and then refuses a
/// 512 m block that needs 23.8 m, which is the same ring at a different size. Half a mean edge
/// leaves 2.2× headroom over both, and [`RingReport::offset_capped`] reports it if it ever binds.
pub const MAX_AREA_OFFSET_EDGES: f64 = 0.5;

/// Pin a corner whose cut would **eat** forest, when the wedge it would eat is this solid — a real
/// promontory. 3× the marching iso; the canopy channel is a box-SUM over a 3×3 8 m window, so this
/// reads "at least six trees within ~24 m of the corner", not "marginally above threshold".
pub const PIN_SOLID_MASS: f64 = 3.0 * CANOPY_MASS_ISO;

/// Pin a corner whose cut would **grow** forest into the wedge, only when that wedge is this bare —
/// a real clearing. Zero: a notch with *any* canopy in it is a 32 m threshold artefact and rounds.
pub const PIN_CLEAR_MASS: f64 = 0.0;

/// Acceptance bound (`T-149.toml`): area drift per region, as a fraction.
pub const MAX_AREA_DRIFT: f64 = 0.03;

const DENSITY_CELL_M_F: f64 = density::DENSITY_CELL_M as f64;

/// Degenerate-geometry epsilon in metres² / metres. Ring coordinates are world metres in the
/// thousands, so anything this small is a duplicate point or a zero-length edge.
const EPS: f64 = 1e-9;

/// The 8 m canopy evidence the corner-preserving option consults: canopy mass at a world (x, y).
///
/// `None` at a call site turns corner preservation off — every corner is then cut. The emit
/// passes `Some`, backed by [`density::sample_corners`] over the same canopy-blurred tree grid
/// that is written to the TBDD tiles.
pub type CanopyMass<'a> = &'a dyn Fn(f64, f64) -> f64;

/// What one ring did. `area_*` are **signed** shoelace areas: an outer ring and a hole carry
/// opposite signs, so summing them over a region gives the region's material area directly.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RingReport {
    pub verts_in: usize,
    pub verts_out: usize,
    /// Vertices the 8 m canopy evidence held sharp.
    pub pinned: usize,
    pub area_in: f64,
    pub area_out: f64,
    /// The uniform normal offset applied to restore the area, in metres (signed).
    pub offset_m: f64,
    /// The offset wanted more than [`MAX_AREA_OFFSET_EDGES`] of a mean edge and was clamped — never seen on everon.
    pub offset_capped: bool,
    /// The ring was below [`MIN_SMOOTH_VERTICES`] and passed through untouched.
    pub skipped_small: bool,
}

/// What one region did — the per-region vertex count and area drift the ticket asks the emit to
/// report.
#[derive(Clone, Debug, Default)]
pub struct RegionReport {
    pub id: String,
    pub rings: usize,
    pub verts_in: usize,
    pub verts_out: usize,
    pub pinned: usize,
    /// Signed ring areas summed — the region's material area, holes subtracted.
    pub area_in: f64,
    pub area_out: f64,
    pub max_offset_m: f64,
    pub offset_capped: usize,
}

impl RegionReport {
    /// Area drift as a fraction of the input area. A region with no area drifts by 0 rather than
    /// by NaN — there is nothing to have drifted.
    #[must_use]
    pub fn area_drift(&self) -> f64 {
        if self.area_in.abs() < EPS {
            return 0.0;
        }
        (self.area_out - self.area_in).abs() / self.area_in.abs()
    }
}

fn round_coord(v: f64) -> f64 {
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
fn left_normal(dx: f64, dy: f64) -> Option<(f64, f64)> {
    let len = dx.hypot(dy);
    if len < EPS {
        return None;
    }
    Some((-dy / len, dx / len))
}

/// Unit vertex normals (the angle bisector of the two adjacent edge normals, on the ring's own
/// "left" side). Falls back to a single edge normal at a spike, and to (0,0) only when both edges
/// are degenerate — an offset of zero there is the safe answer.
fn vertex_normals(v: &[(f64, f64)]) -> Vec<(f64, f64)> {
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
fn corner_pins(v: &[(f64, f64)], sign: f64, canopy: CanopyMass<'_>) -> Vec<bool> {
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

fn lerp(a: (f64, f64), b: (f64, f64), t: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// One corner-preserving Chaikin pass over a closed ring of distinct vertices. A pinned vertex is
/// emitted as itself and neither of the two points that would have cut it is produced, so the
/// pin survives every subsequent iteration.
fn chaikin_once(v: &[(f64, f64)], pin: &[bool]) -> (Vec<(f64, f64)>, Vec<bool>) {
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
fn offset_area_coefficients(v: &[(f64, f64)], u: &[(f64, f64)]) -> (f64, f64, f64) {
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
fn mean_edge(v: &[(f64, f64)]) -> f64 {
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
fn solve_offset(v: &[(f64, f64)], u: &[(f64, f64)], target: f64) -> (f64, bool) {
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
fn dedupe(v: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
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
fn ring_from_json(ring: &Value) -> Option<Vec<(f64, f64)>> {
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

fn ring_to_json(ring: &[(f64, f64)]) -> Value {
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::forest::REGION_CELL_M;

    fn closed(v: &[(f64, f64)]) -> Vec<(f64, f64)> {
        let mut r = v.to_vec();
        r.push(v[0]);
        r
    }

    /// Every turn of `ring` (given closed) that is a right angle, and the largest run of
    /// consecutive right-angle turns. A staircase is not "some 90° turns", it is 90° turns *in a
    /// row* — an isolated sharp corner on an otherwise curved boundary is a preserved feature.
    fn right_angle_turns(ring: &[(f64, f64)]) -> (usize, usize, usize) {
        let mut v = ring.to_vec();
        while v.len() > 1 && v.first() == v.last() {
            v.pop();
        }
        let n = v.len();
        let square: Vec<bool> = (0..n)
            .map(|i| {
                let p = v[(i + n - 1) % n];
                let c = v[i];
                let q = v[(i + 1) % n];
                let (ax, ay) = (c.0 - p.0, c.1 - p.1);
                let (bx, by) = (q.0 - c.0, q.1 - c.1);
                let (la, lb) = (ax.hypot(ay), bx.hypot(by));
                if la < EPS || lb < EPS {
                    return false;
                }
                ((ax * bx + ay * by) / (la * lb)).abs() < 0.05
            })
            .collect();
        let adjacent = (0..n).filter(|i| square[*i] && square[(i + 1) % n]).count();
        (n, square.iter().filter(|s| **s).count(), adjacent)
    }

    fn square_ring(side: f64) -> Vec<(f64, f64)> {
        closed(&[(0.0, 0.0), (side, 0.0), (side, side), (0.0, side)])
    }

    /// A staircase down the diagonal on the 32 m lattice — the shape `trace_rings` actually
    /// emits, and the one a square cannot stand in for (a square has no concave corners).
    fn staircase_ring(steps: usize) -> Vec<(f64, f64)> {
        let c = REGION_CELL_M;
        let mut v = vec![(0.0, 0.0)];
        for i in 0..steps {
            let x = (i + 1) as f64 * c;
            let y = i as f64 * c;
            v.push((x, y));
            v.push((x, y + c));
        }
        v.push((0.0, steps as f64 * c));
        closed(&v)
    }

    #[test]
    fn a_square_ring_rounds() {
        let side = 4.0 * REGION_CELL_M;
        let (out, rep) = chaikin(&square_ring(side), CHAIKIN_ITERATIONS, None);
        assert!(out.first() == out.last(), "the ring must come back closed");
        assert_eq!(rep.verts_in, 4);
        assert_eq!(rep.verts_out, 16, "two Chaikin passes take 4 corners to 16");
        assert!(
            out.len() >= 5,
            "a closed ring needs >= 4 entries + the repeat"
        );

        // ROUNDS: not one right angle survives, and the corners have actually moved.
        let (_, square, adjacent) = right_angle_turns(&out);
        assert_eq!(square, 0, "a rounded ring has no right-angle turns left");
        assert_eq!(adjacent, 0);
        for corner in [(0.0, 0.0), (side, 0.0), (side, side), (0.0, side)] {
            assert!(
                !out.contains(&corner),
                "corner {corner:?} survived the smoother"
            );
        }
        // …and it is a rounding, not a collapse: the ring stays inside the square plus the one
        // area-restoring offset it is allowed.
        let slack = rep.offset_m.abs();
        for p in &out {
            assert!(
                p.0 >= -slack && p.0 <= side + slack && p.1 >= -slack && p.1 <= side + slack,
                "{p:?} left the square by more than the {slack} m area-restoring offset"
            );
        }
        // The rounding is real curvature, not a chamfer: every turn is shallow.
        let (_, sq, _) = right_angle_turns(&out);
        assert_eq!(sq, 0);
    }

    /// The LOCKED carve-out, spelled out because it is in tension with the test above and a
    /// reader will otherwise think one of them is a bug.
    ///
    /// `docs/specs/ideas/t149_forest_smooth.md` LOCKED says "rings under 6 vertices untouched",
    /// and `trace_rings` drops collinear points — so a solid rectangular block of cells, however
    /// large, traces exactly four vertices and the EMIT leaves it square. `chaikin` above rounds
    /// the same ring; `smooth_ring` is where the rule lives.
    #[test]
    fn the_emit_leaves_a_bare_four_vertex_square_alone() {
        let ring = square_ring(4.0 * REGION_CELL_M);
        let (out, rep) = smooth_ring(&ring, None);
        assert!(
            rep.skipped_small,
            "4 distinct verts is under MIN_SMOOTH_VERTICES"
        );
        assert_eq!(out, ring, "the carve-out must return the ring untouched");
        assert_eq!(rep.area_in, rep.area_out);
        assert_eq!(rep.offset_m, 0.0);
    }

    /// The area bound holds at every scale, and the offset rail stays scale-free while doing it.
    ///
    /// An ABSOLUTE rail fails this test: the 40-cell square needs a 59.4 m offset and the 1-cell
    /// square needs 1.49 m, and the two are the same ring — `|d| / mean edge` is 0.2258 at both.
    /// This is the test that would red if someone put a metre constant back.
    #[test]
    fn a_square_ring_holds_its_area_under_the_bound_at_every_scale() {
        let mut ratios: Vec<f64> = Vec::new();
        for side in [REGION_CELL_M, 4.0 * REGION_CELL_M, 40.0 * REGION_CELL_M] {
            let (out, rep) = chaikin(&square_ring(side), CHAIKIN_ITERATIONS, None);
            let drift = (rep.area_out - rep.area_in).abs() / rep.area_in.abs();
            assert!(
                drift < MAX_AREA_DRIFT,
                "side {side}: drift {:.4}% is over the {:.1}% bound",
                drift * 100.0,
                MAX_AREA_DRIFT * 100.0
            );
            assert!(!rep.offset_capped, "side {side}: the offset rail bound");
            let mut v = out.clone();
            v.pop();
            ratios.push(rep.offset_m.abs() / mean_edge(&v));
        }
        for r in &ratios {
            assert!(
                (r - ratios[0]).abs() < 1e-3,
                "the offset/edge ratio moved with scale: {ratios:?}"
            );
            assert!(
                *r < MAX_AREA_OFFSET_EDGES,
                "the square needs {r:.4} edges of offset, rail is {MAX_AREA_OFFSET_EDGES}"
            );
        }
        assert!(
            ratios[0] > 0.2,
            "the offset is not doing real work: {ratios:?}"
        );
    }

    /// The compensation is doing the work — without it a square is >10% light. This is the test
    /// that makes stage 3 non-optional rather than decorative.
    #[test]
    fn uncompensated_chaikin_would_blow_the_bound() {
        let side = 4.0 * REGION_CELL_M;
        let ring = square_ring(side);
        let mut v = ring.clone();
        v.pop();
        let mut pin = vec![false; v.len()];
        for _ in 0..CHAIKIN_ITERATIONS {
            let (nv, np) = chaikin_once(&v, &pin);
            v = nv;
            pin = np;
        }
        let raw_drift = (signed_area(&v) - side * side).abs() / (side * side);
        assert!(
            raw_drift > 0.10,
            "raw Chaikin on a square drifts {:.2}%, expected > 10% — if this ever falls the \
             compensation below is no longer load-bearing and this module should say so",
            raw_drift * 100.0
        );
        let (_, rep) = chaikin(&ring, CHAIKIN_ITERATIONS, None);
        let fixed = (rep.area_out - rep.area_in).abs() / rep.area_in.abs();
        assert!(
            fixed < MAX_AREA_DRIFT,
            "compensated drift {:.4}% is not under the bound",
            fixed * 100.0
        );
        assert!(rep.offset_m.abs() > EPS, "no offset was applied");
    }

    #[test]
    fn a_three_point_ring_stays_valid() {
        let c = REGION_CELL_M;
        let tri = closed(&[(0.0, 0.0), (c, 0.0), (0.0, c)]);
        let (out, rep) = smooth_ring(&tri, None);
        assert!(
            rep.skipped_small,
            "3 distinct verts is under MIN_SMOOTH_VERTICES"
        );
        assert_eq!(
            out, tri,
            "a ring the smoother skips must come back untouched"
        );
        assert!(out.len() >= 4, "the schema wants >= 4 closed-ring entries");
        assert_eq!(out.first(), out.last(), "still closed");
        assert!((signed_area(&out[..out.len() - 1]) - c * c / 2.0).abs() < EPS);

        // The carve-out is on DISTINCT vertices, so a 5-vertex ring is also skipped and a
        // 6-vertex one is not — the boundary is where MIN_SMOOTH_VERTICES says it is.
        let five = closed(&[(0.0, 0.0), (c, 0.0), (2.0 * c, c), (c, 2.0 * c), (0.0, c)]);
        assert!(smooth_ring(&five, None).1.skipped_small);
        let six = closed(&[
            (0.0, 0.0),
            (c, 0.0),
            (2.0 * c, c),
            (2.0 * c, 2.0 * c),
            (c, 2.0 * c),
            (0.0, c),
        ]);
        assert!(!smooth_ring(&six, None).1.skipped_small);
    }

    /// A hole ring: clockwise, so its signed area is negative and it subtracts from the region.
    /// The smoother must round it without flipping its orientation and without pulling it through
    /// itself. An L, not a square, so the ring clears the carve-out and carries a concave corner.
    #[test]
    fn a_hole_ring_keeps_its_sign_and_its_area() {
        let c = REGION_CELL_M;
        let mut l = vec![
            (0.0, 0.0),
            (3.0 * c, 0.0),
            (3.0 * c, c),
            (c, c),
            (c, 3.0 * c),
            (0.0, 3.0 * c),
        ];
        l.reverse(); // clockwise — a hole
        let hole = closed(&l);
        assert!(
            signed_area(&hole[..hole.len() - 1]) < 0.0,
            "the hole ring must be negatively oriented"
        );
        let (out, rep) = smooth_ring(&hole, None);
        assert!(!rep.skipped_small, "a 6-vertex ring must be smoothed");
        assert!(
            rep.area_out < 0.0,
            "the smoother flipped a hole's orientation"
        );
        let drift = (rep.area_out - rep.area_in).abs() / rep.area_in.abs();
        assert!(drift < MAX_AREA_DRIFT, "hole drift {:.4}%", drift * 100.0);
        let (_, square, _) = right_angle_turns(&out);
        assert_eq!(square, 0, "the hole did not round");
    }

    /// The corner-preserving option must be able to FIRE and must CHANGE the output — a pin path
    /// that cannot alter a ring is worse than no pin path at all.
    #[test]
    fn the_canopy_oracle_pins_corners_and_changes_the_ring() {
        let ring = staircase_ring(6);
        let (bare, bare_rep) = smooth_ring(&ring, None);
        assert_eq!(bare_rep.pinned, 0);

        // Solid everywhere: every convex corner is a real promontory, no notch is bare, so only
        // the convex corners pin.
        let solid = |_x: f64, _y: f64| PIN_SOLID_MASS;
        let (dense_out, dense_rep) = smooth_ring(&ring, Some(&solid));
        assert!(dense_rep.pinned > 0, "the solid oracle pinned nothing");
        assert_ne!(dense_out, bare, "pinning did not change the ring");
        assert!(
            dense_out.len() < bare.len(),
            "a pinned corner emits one vertex, not two: {} vs {}",
            dense_out.len(),
            bare.len()
        );

        // Bare everywhere: no convex corner is solid, every notch is bare, so only the concave
        // corners pin — a different set, which proves the convex/concave split is live.
        let bare_field = |_x: f64, _y: f64| 0.0;
        let (clear_out, clear_rep) = smooth_ring(&ring, Some(&bare_field));
        assert!(clear_rep.pinned > 0, "the bare oracle pinned nothing");
        assert_ne!(
            clear_rep.pinned, dense_rep.pinned,
            "the convex and concave branches pinned the same count — one of them is dead"
        );
        assert_ne!(clear_out, dense_out);

        // Marginal canopy — above the clearing floor, below the solid floor — is the 32 m
        // quantisation band, and there NOTHING pins.
        let marginal = |_x: f64, _y: f64| CANOPY_MASS_ISO;
        let (marginal_out, marginal_rep) = smooth_ring(&ring, Some(&marginal));
        assert_eq!(marginal_rep.pinned, 0, "the ambiguous band must not pin");
        assert_eq!(marginal_out, bare);

        // And a pin is exact: the pinned vertices survive verbatim into the output. (Offset by
        // the area compensation, so compare against the input corner within one offset.)
        assert!(dense_rep.offset_m.abs() > EPS && !dense_rep.offset_capped);
        for r in [&dense_rep, &clear_rep] {
            let drift = (r.area_out - r.area_in).abs() / r.area_in.abs();
            assert!(drift < MAX_AREA_DRIFT, "pinned drift {:.4}%", drift * 100.0);
        }
    }

    /// A HOLE ring must read the canopy the other way round, and this is the test that caught it
    /// being wrong.
    ///
    /// The pin rule asks "does the cut eat forest, or grow it?", and a hole encloses a *clearing*:
    /// shrinking a hole's enclosure ADDS forest. So the same ring traversed the other way must pin
    /// the OTHER corners. The first version of `corner_pins` branched on the
    /// orientation-normalised turn, which is invariant under reversal — it gave a hole and its
    /// outer twin identical pins, i.e. it pinned every clearing corner as if the clearing were a
    /// forest. `everon_smooths…` could not see it (a hole with ≥ 6 vertices is rare) and
    /// `a_hole_ring_keeps_its_sign_and_its_area` could not either (it passes no oracle at all).
    #[test]
    fn a_hole_reads_the_canopy_the_other_way_round() {
        let c = REGION_CELL_M;
        let l = vec![
            (0.0, 0.0),
            (3.0 * c, 0.0),
            (3.0 * c, c),
            (c, c),
            (c, 3.0 * c),
            (0.0, 3.0 * c),
        ];
        let outer = closed(&l);
        let mut rev = l.clone();
        rev.reverse();
        let hole = closed(&rev);
        assert!(signed_area(&l) > 0.0 && signed_area(&rev) < 0.0);

        // "Solid everywhere": every forest-eating cut is refused, every forest-growing cut is
        // taken. Reversing the ring swaps which corners those are, so the pin sets are
        // complementary — an L hexagon is 5 convex + 1 reflex (5·90° + 270° = 720°), so 5 pins
        // one way and 1 the other.
        let solid = |_x: f64, _y: f64| PIN_SOLID_MASS;
        let outer_pins = corner_pins(&l, 1.0, &solid);
        let hole_pins = corner_pins(&rev, -1.0, &solid);
        let outer_n = outer_pins.iter().filter(|p| **p).count();
        let hole_n = hole_pins.iter().filter(|p| **p).count();
        assert_eq!(outer_n, 5, "the L has 5 material-convex corners");
        assert_eq!(
            hole_n, 1,
            "reversed, the same 6 corners must pin the complementary 1 — a hole that pins the \
             same corners as its outer twin is reading the canopy as if the clearing were forest"
        );
        assert_eq!(
            outer_n + hole_n,
            l.len(),
            "the two sets must partition the ring"
        );
        for i in 0..l.len() {
            // The reversed ring visits the same vertices in the opposite order.
            let j = rev.iter().position(|p| *p == l[i]).expect("same vertices");
            assert_ne!(
                outer_pins[i], hole_pins[j],
                "vertex {i} {:?} pinned the same way as an outer and as a hole",
                l[i]
            );
        }

        // …and it survives the full pipeline, area and orientation intact.
        let (_, o) = smooth_ring(&outer, Some(&solid));
        let (_, h) = smooth_ring(&hole, Some(&solid));
        assert_eq!((o.pinned, h.pinned), (5, 1));
        assert!(
            o.area_out > 0.0 && h.area_out < 0.0,
            "an orientation flipped"
        );
        for r in [&o, &h] {
            let drift = (r.area_out - r.area_in).abs() / r.area_in.abs();
            assert!(drift < MAX_AREA_DRIFT, "drift {:.4}%", drift * 100.0);
        }
    }

    /// The probe has to reach a DIFFERENT 8 m corner than the vertex, or the oracle is asking
    /// itself. Diagonal bisector, so the per-axis step is `CORNER_PROBE_M / √2`.
    #[test]
    fn the_corner_probe_clears_the_vertexs_own_density_cell() {
        let half = f64::from(density::DENSITY_CELL_M) / 2.0;
        let step = CORNER_PROBE_M / std::f64::consts::SQRT_2;
        assert!(
            step >= half,
            "a {CORNER_PROBE_M} m diagonal probe moves {step:.3} m per axis, inside the ±{half} m \
             corner window — it would resample the vertex's own corner"
        );
        // …and it does not overshoot into the next-but-one cell either.
        assert!(step < 3.0 * half);
    }

    #[test]
    fn the_area_solver_hits_its_target_in_closed_form() {
        // A ring the quadratic term actually matters for.
        let v: Vec<(f64, f64)> = (0..12)
            .map(|i| {
                let a = f64::from(i) * std::f64::consts::TAU / 12.0;
                (100.0 + 60.0 * a.cos(), 100.0 + 35.0 * a.sin())
            })
            .collect();
        let u = vertex_normals(&v);
        for target_scale in [0.90, 0.99, 1.0, 1.01, 1.10] {
            let target = signed_area(&v) * target_scale;
            let (d, capped) = solve_offset(&v, &u, target);
            assert!(!capped, "scale {target_scale}: capped at {d}");
            let moved: Vec<(f64, f64)> = v
                .iter()
                .zip(&u)
                .map(|(p, n)| (p.0 + d * n.0, p.1 + d * n.1))
                .collect();
            let got = signed_area(&moved);
            assert!(
                (got - target).abs() < 1e-6 * target.abs().max(1.0),
                "scale {target_scale}: solved d={d} gives area {got}, wanted {target}"
            );
        }
    }

    /// A ring so degenerate the solver cannot help must not produce NaN coordinates.
    #[test]
    fn a_degenerate_ring_does_not_produce_nan() {
        let collapsed = closed(&[
            (0.0, 0.0),
            (0.0, 0.0),
            (0.0, 0.0),
            (0.0, 0.0),
            (0.0, 0.0),
            (0.0, 0.0),
        ]);
        let (out, rep) = smooth_ring(&collapsed, None);
        assert!(out.iter().all(|p| p.0.is_finite() && p.1.is_finite()));
        assert!(rep.offset_m.is_finite() && rep.area_out.is_finite());
    }

    /* ── the real everon catalogue ── */

    fn everon_objects() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/map-assets/everon/objects")
    }

    /// The committed everon `forest-regions.json.gz`. Neither this nor the density tiles below is
    /// LFS-tracked (`git check-attr filter` → unspecified), so a missing or short corpus is a
    /// FAILURE, never a skip.
    fn everon_regions() -> Vec<Value> {
        let p = everon_objects().join("forest-regions.json.gz");
        let bytes = std::fs::read(&p).unwrap_or_else(|e| panic!("T-149: {} ({e})", p.display()));
        let raw = crate::world::build::gunzip(&bytes).expect("gunzip forest-regions");
        let doc: Value = serde_json::from_slice(&raw).expect("parse forest-regions");
        let regions = doc["regions"].as_array().cloned().unwrap_or_default();
        assert!(
            regions.len() >= 30,
            "everon ships 36 forest regions, found {} — this test would have compared nothing",
            regions.len()
        );
        regions
    }

    /// The 625 committed density tiles reassembled into the global 8 m canopy grid the exporter
    /// blurs and slices. Built from `slice_chunk_corners`' own stride rule, so a stride change
    /// reds here too.
    fn everon_canopy() -> (Vec<u32>, usize) {
        let cols = density::DENSITY_COLS as usize;
        let stride = cols - 1;
        let chunks = 25usize; // everon: 12 800 m / 512 m
        let size = chunks * stride + 1;
        assert_eq!(size, density::corner_grid_size(12_800.0));
        let mut grid = vec![0u32; size * size];
        let dir = everon_objects().join("density");
        let mut tiles = 0usize;
        for cy in 0..chunks {
            for cx in 0..chunks {
                let p = dir.join(format!("{cx}_{cy}.bin"));
                let bytes =
                    std::fs::read(&p).unwrap_or_else(|e| panic!("T-149: {} ({e})", p.display()));
                assert_eq!(
                    bytes.len(),
                    density::TBDD_FILE_BYTES,
                    "{} is {} B — a `vers…` prefix here means an LFS pointer, not a payload",
                    p.display(),
                    bytes.len()
                );
                let g = map_engine_core::geometry::tbdd::decode_tbdd(&bytes)
                    .unwrap_or_else(|e| panic!("{}: {e}", p.display()));
                assert_eq!(
                    (g.cols, g.rows, g.cell_m, g.version),
                    (
                        density::DENSITY_COLS,
                        density::DENSITY_ROWS,
                        density::DENSITY_CELL_M,
                        density::TBDD_VERSION
                    ),
                    "{} header disagrees with this build's constants — the smoother would be \
                     sampling a field it does not understand",
                    p.display()
                );
                for j in 0..density::DENSITY_ROWS as usize {
                    for i in 0..cols {
                        grid[(cy * stride + j) * size + cx * stride + i] =
                            u32::from(g.channels[0][j * cols + i]);
                    }
                }
                tiles += 1;
            }
        }
        assert_eq!(tiles, 625);
        assert!(
            grid.iter().filter(|v| **v > 0).count() > 100_000,
            "the reassembled canopy grid is nearly empty — the oracle below would be vacuous"
        );
        (grid, size)
    }

    /// T-149 ACCEPTANCE, on the whole committed everon catalogue rather than a sample.
    ///
    /// Three claims, all measured against the shipped rings:
    ///   1. every region's area drift is under 3%;
    ///   2. the boundary stops being a staircase — and this is checked by ATTRIBUTION, not by a
    ///      tolerance: after smoothing, a right-angle turn may only exist on a ring the LOCKED
    ///      `MIN_SMOOTH_VERTICES` carve-out protected, or at a vertex the 8 m canopy pinned. Any
    ///      other surviving right angle is leftover staircase and reds here.
    ///   3. `smooth_regions` is exactly `smooth_ring` plus JSON — the emit path cannot quietly
    ///      differ from the geometry these claims are proved against.
    #[test]
    fn everon_smooths_within_the_area_bound_and_stops_being_a_staircase() {
        let (grid, size) = everon_canopy();
        let canopy = |x: f64, y: f64| f64::from(density::sample_corners(&grid, size, x, y));
        let regions_in = everon_regions();

        // BEFORE: the defect, quantified. And, per ring, what the smoother is allowed to leave
        // square: everything on a carved-out ring, and the pins on every other.
        let (mut turns_in, mut square_in, mut adjacent_in) = (0, 0, 0);
        let (mut turns_out, mut square_out, mut adjacent_out, mut allowed_square) = (0, 0, 0, 0);
        let (mut carved_rings, mut carved_verts) = (0, 0);
        let mut worst_offset_edges = 0.0f64;
        let mut per_ring: Vec<Vec<(f64, f64)>> = Vec::new();
        for r in &regions_in {
            for ring_json in r["polygon"].as_array().expect("polygon") {
                let ring = ring_from_json(ring_json).expect("finite ring");
                let (n, sq, adj) = right_angle_turns(&ring);
                turns_in += n;
                square_in += sq;
                adjacent_in += adj;

                let (out, rep) = smooth_ring(&ring, Some(&canopy));
                if !rep.skipped_small {
                    let mut distinct = out.clone();
                    distinct.pop();
                    let edge = mean_edge(&distinct);
                    assert!(edge > EPS);
                    worst_offset_edges = worst_offset_edges.max(rep.offset_m.abs() / edge);
                }
                assert!(
                    out.len() >= 4 && out.first() == out.last(),
                    "{}: a smoothed ring stopped being a valid closed ring",
                    r["id"]
                );
                let (n2, sq2, adj2) = right_angle_turns(&out);
                turns_out += n2;
                square_out += sq2;
                adjacent_out += adj2;
                if rep.skipped_small {
                    carved_rings += 1;
                    carved_verts += n2;
                    allowed_square += n2;
                    assert_eq!(out, ring, "a carved-out ring was modified");
                } else {
                    allowed_square += rep.pinned;
                    assert!(
                        sq2 <= rep.pinned,
                        "{}: {sq2} right angles survived on a ring with only {} pins — \
                         that is leftover staircase",
                        r["id"],
                        rep.pinned
                    );
                }
                per_ring.push(out);
            }
        }
        assert_eq!(
            (square_in, adjacent_in),
            (turns_in, turns_in),
            "the shipped rings are supposed to be 100% right angles ({turns_in} turns)"
        );
        assert!(carved_rings > 0 && carved_verts > 0);
        // The rail has real headroom rather than merely not binding — this is the claim
        // MAX_AREA_OFFSET_EDGES is set from, held against the corpus instead of a doc comment.
        assert!(
            worst_offset_edges < 0.5 * MAX_AREA_OFFSET_EDGES,
            "the hungriest everon ring wants {worst_offset_edges:.4} of a mean edge, rail is \
             {MAX_AREA_OFFSET_EDGES} — less than 2x headroom left"
        );
        assert!(worst_offset_edges > 0.0, "no ring needed any offset at all");

        // CLAIM 3 — run the production path and hold it to the same rings.
        let mut regions = regions_in.clone();
        let reports = smooth_regions(&mut regions, Some(&canopy));
        assert_eq!(reports.len(), regions.len());
        let mut k = 0usize;
        for r in &regions {
            for ring_json in r["polygon"].as_array().expect("polygon") {
                let got = ring_from_json(ring_json).expect("finite ring");
                assert_eq!(got, per_ring[k], "smooth_regions diverged from smooth_ring");
                k += 1;
            }
        }
        assert_eq!(k, per_ring.len());

        // CLAIM 1 — area.
        let mut worst = (0.0f64, String::new());
        let (mut vin, mut vout, mut pinned) = (0, 0, 0);
        for r in &reports {
            assert!(r.rings > 0, "{}: no rings were read", r.id);
            assert_eq!(r.offset_capped, 0, "{}: the offset rail bound", r.id);
            // A measured property of this corpus, not the rail: on everon the whole boundary
            // never has to move by even one 8 m density cell to give the area back.
            assert!(
                r.max_offset_m < DENSITY_CELL_M_F,
                "{}: offset {:.3} m is a whole density cell — the compensation is doing more \
                 than the smoothing",
                r.id,
                r.max_offset_m
            );
            let d = r.area_drift();
            if d > worst.0 {
                worst = (d, r.id.clone());
            }
            assert!(
                d < MAX_AREA_DRIFT,
                "{}: area drift {:.4}% is over the {:.1}% acceptance bound",
                r.id,
                d * 100.0,
                MAX_AREA_DRIFT * 100.0
            );
            vin += r.verts_in;
            vout += r.verts_out;
            pinned += r.pinned;
        }
        assert!(vout > vin, "the smoother added no vertices at all");
        assert!(
            pinned > 0,
            "the canopy oracle pinned nothing across all 36 regions — on this corpus the \
             corner-preserving path would be dead code"
        );

        // CLAIM 2 — the staircase is gone. Every surviving right angle is accounted for, and the
        // consecutive-right-angle signature that defines a staircase has collapsed.
        // `<=` and not `==`: a pin does not always LEAVE a right angle (two adjacent pins, or a
        // pin on a collinear run, come out shallow). The exact claim is the per-ring
        // `sq2 <= rep.pinned` above; this is its roll-up.
        assert!(
            square_out <= allowed_square,
            "{square_out} right angles survived, only {allowed_square} are attributable to the \
             carve-out ({carved_verts} verts) plus canopy pins ({pinned})"
        );
        assert!(square_out > 0, "the attribution above compared nothing");
        assert!(
            adjacent_out * 10 < turns_out,
            "{adjacent_out} of {turns_out} turns are still consecutive right angles (>10%)"
        );

        eprintln!(
            "[T-149] everon: {} regions, {vin} -> {vout} verts, {pinned} pinned, \
             {carved_rings} rings carved out ({carved_verts} verts); \
             right-angle turns {square_in}/{turns_in} -> {square_out}/{turns_out}, \
             consecutive {adjacent_in} -> {adjacent_out}; worst area drift {:.6}% ({}) \
             vs bound {:.1}%; largest area-restoring offset {:.4} m \
             ({worst_offset_edges:.4} of a mean edge, rail {MAX_AREA_OFFSET_EDGES})",
            reports.len(),
            worst.0 * 100.0,
            worst.1,
            MAX_AREA_DRIFT * 100.0,
            reports.iter().fold(0.0f64, |m, r| m.max(r.max_offset_m))
        );
    }
}
