//! Chaikin smoothing for the Path B forest rings.
//!
//! `forest::trace_rings` walks cell boundaries on the 32 m region lattice, so **every** emitted
//! ring segment is axis-aligned and every turn is exactly ±90°: measured on the committed everon
//! catalogue, 15 528 / 15 528 segments (100.00%). That is the "blocky" this module fixes.
//!
//! Three stages, in order, per ring:
//!
//! 1. **Corner pins** (the corner-preserving option) — decided ONCE on the input ring, where the
//!    8 m density grid is meaningful, then carried through every Chaikin iteration.
//! 2. **Chaikin**, `CHAIKIN_ITERATIONS` iterations at cut `CHAIKIN_CUT`, pinned vertices
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
//! the evidence is unambiguous (`PIN_SOLID_MASS` / `PIN_CLEAR_MASS`); the ambiguous band in
//! between is exactly where the 32 m quantisation error lives, and there the ring rounds.
//! Note that the question is about MATERIAL, not about the ring: a hole encloses a clearing, so
//! shrinking a hole's enclosure *adds* forest, and a hole must read the field the other way round
//! from its outer twin (`a_hole_reads_the_canopy_the_other_way_round`).
//!
//! The TBDD format, its header and its writers are untouched — this module only ever *reads*
//! `density::sample_corners`.

use serde_json::Value;
use website_map_engine::world::environment::vegetation::mass::CANOPY_MASS_ISO;

use crate::world_export_pipeline::forest_contours::js_num;
use crate::world_export_pipeline::vegetation_density as density;

/// Chaikin corner cut: the two replacement points sit at `t` and `1 - t` along each segment.
pub const CHAIKIN_CUT: f64 = 0.25;

/// Iterations run at the emit (the ticket's "two iterations").
pub const CHAIKIN_ITERATIONS: usize = 2;

/// LOCKED (`documentation_v2/tickets/specs/t149_forest_smooth.md`): a ring with fewer
/// *distinct* vertices than
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

/// Acceptance bound: area drift per region, as a fraction.
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

#[cfg(test)]
#[path = "tests/forest_smoothing/tests.rs"]
mod tests;

#[path = "forest_smoothing/round_coord.rs"]
mod round_coord;
pub use round_coord::chaikin;
pub use round_coord::log_reports;
pub use round_coord::signed_area;
pub use round_coord::smooth_regions;
pub use round_coord::smooth_ring;

#[cfg(test)]
pub(crate) use round_coord::mean_edge;

#[cfg(test)]
pub(crate) use round_coord::chaikin_once;

#[cfg(test)]
pub(crate) use round_coord::corner_pins;

#[cfg(test)]
pub(crate) use round_coord::vertex_normals;

#[cfg(test)]
pub(crate) use round_coord::solve_offset;

#[cfg(test)]
pub(crate) use round_coord::ring_from_json;
