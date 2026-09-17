use super::*;
use crate::v2::apps::editor::shell::layout::{DOCK_LEFT_PX, DOCK_RIGHT_PX, STRIP_TOP_PX};

/// The editor's real camera build (`select_tool::frozen_camera`): Everon bounds `[0,0,12800,
/// 12800]`, north-up, no rotation — so the projection the labels use is the one the GPU grid and
/// the CUR readout use.
fn cam(width: f64, height: f64, tx: f64, ty: f64, zoom: f64) -> OrthoCamera {
    let mut c = OrthoCamera::new(width, height, tx, ty, zoom);
    c.set_bounds(0.0, 0.0, 12_800.0, 12_800.0);
    c
}

/// The km index each easting label names, recovered from its text (`"064"` → 6). The label's
/// world line is `k·1000` metres; this lets the test recompute `screen_x(k·1000)` independently
/// of `pos_px` for the `≤ 2 px` acceptance check.
fn km_index(text: &str) -> i64 {
    // 3-digit hundreds-of-metres → metres → km. `"064"` = 6400 m = km 6.
    text.parse::<i64>().expect("3-digit ref") * 100 / 1000
}

/// ACCEPTANCE — 5 pans × 3 zooms: every visible easting sits within 2 px of its km line
/// (CUR-unproject oracle), and adjacent labels are exactly `1000 / m_per_px` px apart.
#[test]
fn labels_track_the_live_camera_across_pans_and_zooms() {
    let (w, h) = (1600.0, 900.0);
    let pane_right = w - DOCK_RIGHT_PX;
    // 3 scripted zoom levels (m/px = 2^-zoom: 4.0, 1.0, 0.5) and 5 scripted pans (world targets).
    let zooms = [-2.0_f64, 0.0, 1.0];
    let pans = [
        (6400.0_f64, 6400.0_f64), // centre
        (6640.0, 6400.0),         // +240 m east — the review's failing pan distance
        (3000.0, 9000.0),         // NW-ish
        (9500.0, 2500.0),         // SE-ish
        (5120.0, 7680.0),         // off-centre
    ];
    for z in zooms {
        let mpp = m_per_px(z);
        let step_px = 1000.0 / mpp; // km-line spacing in screen px at this zoom
        for (tx, ty) in pans {
            let c = cam(w, h, tx, ty, z);
            let eastings = edge_eastings(&c, DOCK_LEFT_PX, pane_right, STRIP_TOP_PX);
            // (a) Each label is on its km line to within 2 px — recompute screen_x(k·1000) via
            //     the SAME camera and compare to the emitted label_x. The oracle: unprojecting
            //     that screen x returns k·1000 (the trusted CUR round-trip).
            for lbl in &eastings {
                let k = km_index(&lbl.text);
                let screen_x = c.project([k as f64 * 1000.0, c.target_y(), 0.0])[0];
                assert!(
                    (screen_x - lbl.pos_px).abs() <= 2.0,
                    "z={z} pan=({tx},{ty}): label {} at {:.2}px is >2px off its km line's \
                     screen x {screen_x:.2}",
                    lbl.text,
                    lbl.pos_px
                );
                // CUR-unproject oracle: the label's own x unprojects back to its km line.
                let wx = c.unproject_xy(lbl.pos_px, STRIP_TOP_PX)[0];
                assert!(
                    (wx - k as f64 * 1000.0).abs() <= 2.0 * mpp,
                    "z={z}: label {} unprojects to world x {wx:.1}, not its km line {}",
                    lbl.text,
                    k * 1000
                );
            }
            // (b) Adjacent labels are exactly one km-line spacing apart (1000 / m_per_px px).
            //     Labels come out in ascending world x → ascending screen x (north-up, no rot).
            for pair in eastings.windows(2) {
                let gap = pair[1].pos_px - pair[0].pos_px;
                assert!(
                    (gap - step_px).abs() <= 0.5,
                    "z={z} pan=({tx},{ty}): {} and {} are {gap:.1}px apart; km lines must be \
                     {step_px:.1}px ({mpp} m/px). This is the O-2 arithmetic: 70px @ 4m/px is \
                     two labels that cannot both be true.",
                    pair[0].text,
                    pair[1].text
                );
            }
            // Northings satisfy the same spacing on the Y axis.
            let northings = edge_northings(&c, DOCK_LEFT_PX, STRIP_TOP_PX, h);
            for pair in northings.windows(2) {
                let gap = (pair[1].pos_px - pair[0].pos_px).abs();
                assert!(
                    (gap - step_px).abs() <= 0.5,
                    "z={z} pan=({tx},{ty}): northings {} and {} are {gap:.1}px apart; must be \
                     {step_px:.1}px",
                    pair[0].text,
                    pair[1].text
                );
            }
        }
    }
}

/// ACCEPTANCE — under continuous pan the set updates every frame. Two mid-pan samples of the
/// SAME visible ref must differ correctly: its screen x moves by the pan delta, and — the fix —
/// its `<For>` key changes, so Leptos cannot reuse the pre-pan node with a frozen `left:` (the
/// O-2 half-update). A label whose position was cached across the pan would keep both.
#[test]
fn continuous_pan_moves_labels_and_busts_the_for_key() {
    let (w, h) = (1600.0, 900.0);
    let pane_right = w - DOCK_RIGHT_PX;
    let z = -2.0; // 4 m/px — the review's zoom
    let mpp = m_per_px(z);
    // Two frames of a continuous eastward pan: the camera target moves +120 m between samples.
    // Target east ⇒ content scrolls WEST, so a fixed grid line's screen x DROPS by 120/mpp =
    // 30 px (`project`: a larger target_x puts the same world x further left on screen).
    let c0 = cam(w, h, 6400.0, 6400.0, z);
    let c1 = cam(w, h, 6520.0, 6400.0, z);
    let a = edge_eastings(&c0, DOCK_LEFT_PX, pane_right, STRIP_TOP_PX);
    let b = edge_eastings(&c1, DOCK_LEFT_PX, pane_right, STRIP_TOP_PX);
    let expected_shift = -120.0 / mpp; // −30 px screen shift (target east ⇒ line x shrinks)
                                       // At least one ref is visible in BOTH frames (a 30 px pan keeps most refs on screen).
    let mut checked = 0;
    for la in &a {
        if let Some(lb) = b.iter().find(|x| x.text == la.text) {
            // Frame-to-frame the label MOVED by the pan delta…
            let moved = lb.pos_px - la.pos_px;
            assert!(
                (moved - expected_shift).abs() <= 0.5,
                "ref {} moved {moved:.1}px between frames; a live label must move by the pan \
                 delta {expected_shift:.1}px, not hold position (the O-2 stall)",
                la.text
            );
            // …and its `<For>` key changed, so the DOM node is re-created at the new x rather
            // than reused with a stale `left:` (the actual O-2 fix — text-keyed nodes did not).
            assert_ne!(
                la.key, lb.key,
                "ref {} kept its <For> key across the pan — a text key would, and Leptos would \
                 then reuse the node and FREEZE its left: at the old x (the O-2 half-update)",
                la.text
            );
            checked += 1;
        }
    }
    assert!(
        checked > 0,
        "the pan must keep at least one ref visible across both frames to compare"
    );
}

/// FIRE THE KEYING RULE (perturb / fail / restore): the position-keyed identity genuinely
/// discriminates. A key that were the TEXT (the reverted defect) would be EQUAL across a pan —
/// the very thing that let Leptos freeze the node. This asserts the real key is NOT equal to the
/// text-only key across a move, so the fix is load-bearing, not incidental.
#[test]
fn keying_rule_fires() {
    let (w, h) = (1600.0, 900.0);
    let pane_right = w - DOCK_RIGHT_PX;
    let z = -2.0;
    let c0 = cam(w, h, 6400.0, 6400.0, z);
    let c1 = cam(w, h, 6520.0, 6400.0, z); // +120 m pan
    let a = edge_eastings(&c0, DOCK_LEFT_PX, pane_right, STRIP_TOP_PX);
    let b = edge_eastings(&c1, DOCK_LEFT_PX, pane_right, STRIP_TOP_PX);
    let la = a.first().expect("some easting visible");
    let lb = b
        .iter()
        .find(|x| x.text == la.text)
        .expect("same ref visible after a 30px pan");
    // Baseline: the real (position) key differs across the pan — the node busts, position tracks.
    assert_ne!(
        la.key, lb.key,
        "the live key must change when the label moves"
    );
    // Perturb: the DEFECT key is the text, which is IDENTICAL across the pan…
    assert_eq!(
        la.text, lb.text,
        "the ref text is unchanged by a pan — which is exactly why text is an unsafe <For> key"
    );
    // …so a build that keyed on text would compare equal here and reuse the stale node. The fix
    // is that our key does NOT: restore-check that the position component is what breaks the tie.
    assert!(
        la.key.ends_with(&la.text) && lb.key.ends_with(&lb.text),
        "the key still carries the ref for disambiguation…"
    );
    assert_ne!(
        la.key, lb.key,
        "…but the pixel prefix makes it bust on movement (restore: the rule still holds)"
    );
}
