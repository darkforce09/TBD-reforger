//! T-151 OrthoCamera property tests (plan §S3b) — seeded, deterministic, 13k+ cases.
//!
//! These are the interaction/consistency invariants the render engine and the spike page
//! rely on; the deck-parity gates live in `deckgl_ortho_parity.rs`.

use map_engine_core::camera::{MAX_ZOOM, MIN_ZOOM, OrthoCamera};

/// Knuth 64-bit LCG; `unit()` yields f64 in [0, 1) from the top 53 bits. Seeded with the
/// house constant so every run is identical.
struct Lcg(u64);

impl Lcg {
    const SEED: u64 = 0x1234_5678;

    fn new() -> Self {
        Self(Self::SEED)
    }

    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / 9_007_199_254_740_992.0 // 2^53
    }

    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.unit() * (hi - lo)
    }
}

fn random_camera(rng: &mut Lcg) -> OrthoCamera {
    OrthoCamera::new(
        rng.range(1.0, 4096.0),
        rng.range(1.0, 4096.0),
        rng.range(-20_000.0, 20_000.0),
        rng.range(-20_000.0, 20_000.0),
        rng.range(MIN_ZOOM, MAX_ZOOM),
    )
}

fn assert_close(got: f64, expected: f64, what: &str, i: usize) {
    let tol = 1e-9 * expected.abs().max(1.0);
    assert!(
        (got - expected).abs() <= tol,
        "case {i}: {what} |{got} - {expected}| > {tol}"
    );
}

/// 10,000 round trips: `unproject(project(p)) ≈ p` within 1e-9 relative, plus the screen
/// center property `project(target) ≈ (w/2, h/2)` (matrix-path cancellation is inexact in
/// floating point, hence the tolerance rather than `==`).
#[test]
fn round_trip_and_center_10k() {
    let mut rng = Lcg::new();
    for i in 0..10_000 {
        let cam = random_camera(&mut rng);
        let p = [
            rng.range(-20_000.0, 20_000.0),
            rng.range(-20_000.0, 20_000.0),
        ];
        let projected = cam.project([p[0], p[1], 0.0]);
        let rt = cam.unproject_xy(projected[0], projected[1]);
        assert_close(rt[0], p[0], "roundTrip.x", i);
        assert_close(rt[1], p[1], "roundTrip.y", i);

        let [w, h] = cam.size_px();
        let center = cam.project([cam.target_x(), cam.target_y(), 0.0]);
        assert_close(center[0], w / 2.0, "center.px", i);
        assert_close(center[1], h / 2.0, "center.py", i);
    }
}

/// 1,000 pan invariants: after `pan(dx, dy)` (no bounds set), every projected point shifts
/// by exactly the screen delta, to 1e-9 relative (FP distributivity keeps this from being
/// bit-exact — documented in the plan).
#[test]
fn pan_shifts_projection_1k() {
    let mut rng = Lcg::new();
    for i in 0..1_000 {
        let mut cam = random_camera(&mut rng);
        let p = [
            rng.range(-20_000.0, 20_000.0),
            rng.range(-20_000.0, 20_000.0),
            0.0,
        ];
        let before = cam.project(p);
        let dx = rng.range(-500.0, 500.0);
        let dy = rng.range(-500.0, 500.0);
        cam.pan(dx, dy);
        let after = cam.project(p);
        assert_close(after[0], before[0] + dx, "pan.px", i);
        assert_close(after[1], before[1] + dy, "pan.py", i);
    }
}

/// 1,000 zoom_at invariants: the world point under the cursor is fixed across the zoom
/// (1e-9 relative), and zoom is clamped to [MIN_ZOOM, MAX_ZOOM].
#[test]
fn zoom_at_fixes_cursor_and_clamps_1k() {
    let mut rng = Lcg::new();
    for i in 0..1_000 {
        let mut cam = random_camera(&mut rng);
        let [w, h] = cam.size_px();
        let cx = rng.range(0.0, w);
        let cy = rng.range(0.0, h);
        let world_before = cam.unproject_xy(cx, cy);
        let dz = rng.range(-14.0, 14.0); // deliberately overshoots the clamp band
        cam.zoom_at(dz, cx, cy);
        assert!(
            (MIN_ZOOM..=MAX_ZOOM).contains(&cam.zoom()),
            "case {i}: zoom {} escaped clamp",
            cam.zoom()
        );
        let world_after = cam.unproject_xy(cx, cy);
        assert_close(world_after[0], world_before[0], "zoomAt.wx", i);
        assert_close(world_after[1], world_before[1], "zoomAt.wy", i);
    }
}

/// 1,000 visible_world_rect consistency checks: min ≤ max per axis, and re-projecting every
/// rect corner lands on the viewport boundary (within 1e-9 relative of [0,w]×[0,h]).
#[test]
fn visible_rect_consistency_1k() {
    let mut rng = Lcg::new();
    for i in 0..1_000 {
        let cam = random_camera(&mut rng);
        let [w, h] = cam.size_px();
        let [min_x, min_y, max_x, max_y] = cam.visible_world_rect();
        assert!(min_x <= max_x && min_y <= max_y, "case {i}: rect inverted");
        let tol_x = 1e-9 * w.max(1.0);
        let tol_y = 1e-9 * h.max(1.0);
        for corner in [
            [min_x, min_y],
            [min_x, max_y],
            [max_x, min_y],
            [max_x, max_y],
        ] {
            let px = cam.project([corner[0], corner[1], 0.0]);
            assert!(
                (-tol_x..=w + tol_x).contains(&px[0]) && (-tol_y..=h + tol_y).contains(&px[1]),
                "case {i}: rect corner {corner:?} projects off-viewport at {px:?}"
            );
        }
    }
}

/// Bounds clamp mirrors the view-state layer: with terrain bounds set, pan/zoom_at/set_view
/// keep the target inside `[0, 12800]²` (Everon).
#[test]
fn bounds_clamp_target_1k() {
    let mut rng = Lcg::new();
    for i in 0..1_000 {
        let mut cam = random_camera(&mut rng);
        cam.set_bounds(0.0, 0.0, 12_800.0, 12_800.0);
        cam.pan(rng.range(-1e6, 1e6), rng.range(-1e6, 1e6));
        cam.zoom_at(rng.range(-3.0, 3.0), 10.0, 10.0);
        cam.set_view(
            rng.range(-30_000.0, 30_000.0),
            rng.range(-30_000.0, 30_000.0),
            0.0,
        );
        let (tx, ty) = (cam.target_x(), cam.target_y());
        assert!(
            (0.0..=12_800.0).contains(&tx) && (0.0..=12_800.0).contains(&ty),
            "case {i}: target ({tx}, {ty}) escaped bounds"
        );
    }
}
