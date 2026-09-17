//! Door hit-testing and active-floor LOS ray geometry for the building viewer.

use super::*;

/// The door leaf under local plan point `p` on a view of `band`: the leaf where it hangs now, or
/// its closed footprint (the aperture) — either counts, so an open door closes on click too.
#[must_use]
pub fn door_at(c: &CompoundBuilding, p: [f64; 2], band: [f64; 2]) -> Option<String> {
    let inside = |inst: &Instance, frame: &Rigid| {
        let q = frame.inverse().point([p[0], 0.0, p[1]]);
        let (lo, hi) = inst.bounds;
        q[0] >= lo[0] - DOOR_HIT_SLACK_M
            && q[0] <= hi[0] + DOOR_HIT_SLACK_M
            && q[2] >= lo[2] - DOOR_HIT_SLACK_M
            && q[2] <= hi[2] + DOOR_HIT_SLACK_M
    };
    c.doors()
        .filter(|inst| instance_in_band(inst, band))
        .find(|inst| inside(inst, &inst.placement()) || inside(inst, &inst.local))
        .map(|inst| inst.record.id.clone())
}

/// Ray strip + event dots for `INTERIOR_PROBE`, clipped to the ACTIVE view's elevation band
/// (`band`/`band_last` from [`ViewFloor::band`], intersected via the raycaster's own
/// [`clip_t_to_band`] so display and evaluation cannot disagree). Spans between consecutive
/// hits are coloured by a state machine over the trace: clear → green, after glass → cyan,
/// after canopy → yellow-green, after furniture cover → yellow; from a terminal block (wall,
/// roof, solid, leaf, frame, prop, full cover) to the target → red. Dots draw only where the
/// hit's own elevation lies inside the band. Returns `(packed, item_count)` — empty when the ray
/// never enters the band (that floor's plan honestly shows no ray).
#[must_use]
pub fn build_ray_lane(
    obs: [f64; 3],
    tgt: [f64; 3],
    hits: &[LosHit],
    is_clear: bool,
    band: [f64; 2],
    band_last: bool,
) -> (Vec<f32>, u32) {
    let mut packed = Vec::new();
    let mut count = 0u32;
    let Some((band_t0, band_t1)) = clip_t_to_band(obs[1], tgt[1], band, band_last) else {
        return (packed, count);
    };
    let o2 = [obs[0], obs[2]];
    let t2 = [tgt[0], tgt[2]];
    let at = |t: f64| [o2[0] + t * (t2[0] - o2[0]), o2[1] + t * (t2[1] - o2[1])];

    let mut spans: Vec<(f64, f64, [f32; 4])> = Vec::new();
    let mut color = RAY_CLEAR;
    let mut t_prev = 0.0f64;
    for h in hits {
        spans.push((t_prev, h.t, color));
        t_prev = h.t;
        color = match h.kind {
            LosHitKind::Wall | LosHitKind::Roof | LosHitKind::Solid => RAY_BLOCKED,
            // A terminal window / stairs hit is frame or tread MASS, not a pass.
            LosHitKind::Window | LosHitKind::Stairs if h.concealment >= 1.0 => RAY_BLOCKED,
            LosHitKind::Furniture if h.concealment >= 1.0 => RAY_BLOCKED,
            LosHitKind::Window | LosHitKind::Glass => RAY_GLASS,
            LosHitKind::Foliage => RAY_FOLIAGE,
            LosHitKind::Furniture => RAY_COVER,
            LosHitKind::DoorOpen | LosHitKind::DoorAperture | LosHitKind::Stairs => color,
            LosHitKind::DoorLeaf
            | LosHitKind::DoorFrame
            | LosHitKind::WindowFrame
            | LosHitKind::Prop => RAY_BLOCKED,
        };
    }
    spans.push((t_prev, 1.0, if is_clear { color } else { RAY_BLOCKED }));

    for (a, b, col) in spans {
        let (a, b) = (a.max(band_t0), b.min(band_t1));
        if b - a < 1e-6 {
            continue;
        }
        let verts = expand_polyline_strip(&[to_world(at(a)), to_world(at(b))], 0.16, col);
        push_strip(&mut packed, &verts);
        count += 1;
    }
    // Event dots — only those inside the viewed band.
    for h in hits {
        if h.pos[1] < band[0] - 1e-9 || h.pos[1] > band[1] + 1e-9 {
            continue;
        }
        let col = match h.kind {
            LosHitKind::Wall | LosHitKind::Roof | LosHitKind::Solid => RAY_BLOCKED,
            LosHitKind::Window | LosHitKind::Stairs if h.concealment >= 1.0 => RAY_BLOCKED,
            LosHitKind::Window | LosHitKind::Glass => RAY_GLASS,
            LosHitKind::DoorOpen | LosHitKind::DoorAperture => RAY_CLEAR,
            LosHitKind::Foliage => RAY_FOLIAGE,
            LosHitKind::Furniture => RAY_COVER,
            LosHitKind::Stairs => COL_HATCH,
            LosHitKind::DoorLeaf
            | LosHitKind::DoorFrame
            | LosHitKind::WindowFrame
            | LosHitKind::Prop => RAY_BLOCKED,
        };
        let c = [h.pos[0], h.pos[2]];
        quad(&mut packed, rect_corners(c, [0.34, 0.34], 45.0), col);
        count += 1;
    }
    (packed, count)
}
