use super::{format_m_per_px, m_per_px, pick_scale_bar};
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body, only_item};
use website_map_engine::camera::ortho::state::MAX_ZOOM;
use website_map_engine::camera::ortho::state::MIN_ZOOM;

/// The readout across the whole zoom clamp, at the real rungs the operator sees. `MIN_ZOOM −6`
/// is whole-Everon (64 m/px), `−2` the editor default (4 m/px), `0` unity, `MAX_ZOOM 6` the
/// close-inspection ceiling (0.0156 m/px). Three significant figures throughout — including
/// below 0.1 m/px, where a fixed 3-decimal format would have dropped to two.
#[test]
fn readout_table_across_the_zoom_clamp() {
    let cases = [
        (MIN_ZOOM, "64.0 m/px"),
        (-4.0, "16.0 m/px"),
        (-2.0, "4.00 m/px"),
        (-1.0, "2.00 m/px"),
        (0.0, "1.00 m/px"),
        (2.0, "0.250 m/px"),
        (4.0, "0.0625 m/px"),
        (MAX_ZOOM, "0.0156 m/px"),
    ];
    for (z, want) in cases {
        let got = format_m_per_px(m_per_px(z));
        assert_eq!(got, want, "zoom {z} must read {want}, got {got}");
    }
    // A degenerate scale reads as the same em-dash "no value" the other cells use — never NaN,
    // never `inf`, on the operator's screen.
    for bad in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
        assert!(
            format_m_per_px(bad).starts_with('\u{2014}'),
            "degenerate m/px {bad} must render the em-dash cell, not a raw float"
        );
    }
    // T-756 (MINOR-4): a non-finite *zoom* must also hit the em-dash — `m_per_px` used to
    // fabricate 1.0 and print a confident "1.00 m/px".
    for bad_z in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let got = format_m_per_px(m_per_px(bad_z));
        assert!(
            got.starts_with('\u{2014}'),
            "non-finite zoom {bad_z} must render the em-dash cell, got {got}"
        );
    }
    // T-756 (MINOR-4): band-top carry must not print four significant figures.
    assert_eq!(format_m_per_px(9.996), "10.0 m/px");
    assert_eq!(format_m_per_px(99.96), "100 m/px");
    assert_eq!(format_m_per_px(0.09996), "0.100 m/px");
}

/// The readout is MONOTONE in zoom: zooming in never prints a larger metres-per-pixel. A
/// formatter that rounded into a non-monotone sequence would make the number lie about the
/// direction of a gesture, which is worse than printing nothing.
#[test]
fn readout_never_goes_backwards_as_you_zoom_in() {
    let mut prev = f64::INFINITY;
    let mut z = MIN_ZOOM;
    while z <= MAX_ZOOM {
        let shown: f64 = format_m_per_px(m_per_px(z))
            .trim_end_matches(" m/px")
            .parse()
            .expect("the readout must be a parseable number plus its unit");
        assert!(
            shown <= prev,
            "zoom {z}: printed {shown} m/px after {prev} — the readout must not increase as \
             you zoom IN"
        );
        prev = shown;
        z += 0.25;
    }
}

/// **Reconciliation with T-639 (wave 101 + T-755).** The summary says this readout is the
/// on-screen check for the zoom-adaptive contour ladder, so it must print the ladder's OWN
/// scale, not a lookalike. `apps/website/map-engine/src/world/terrain/relief/host.rs`
/// `push_contours` computes `2.0_f64.powf(-zoom)` and hands it — with nothing in between — to
/// `contour_interval_for_zoom`; [`m_per_px`] is that same expression (param name `deck_zoom`).
///
/// `contour_interval_for_zoom` itself cannot be CALLED from here — `map-engine-core`'s `world`
/// feature is a wasm32-only dependency of this crate, so on native it does not exist. So the
/// identity is pinned three ways that need no such call: (a) OUR conversion's scrubbed body is
/// `2^(-deck_zoom)`; (b) the ladder's scrubbed body binds that expression and feeds it to the
/// interval selector with no adjustment line between; (c) numerically, our fn matches `2^(-z)`
/// across the clamp (and the printed string stays within display precision). Wave-115 MINOR-3:
/// the old exact-string needles missed an adjustment inserted between the bind and the call,
/// and an upstream `zoom` re-bind; the contiguous feed + no-rebind checks close those holes.
#[test]
fn the_printed_scale_is_the_contour_ladders_own_scale() {
    // (1) OUR conversion — scrubbed body, not a test-local recomputation of the formula alone.
    let ours = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/toolbelt.rs"
    )));
    let our_mpp = only_body(&ours, &format!("pub fn {}", "m_per_px("));
    assert!(
        our_mpp.contains(&format!("2.0_f64.{}(-deck_zoom)", "powf")),
        "T-670/T-755: m_per_px must be 2^(-deck_zoom) — the contour ladder's screen-scale              convention"
    );

    // (2) THE LADDER's feed — frontend dem_vectors.rs (not crates/). Contiguous bind→call so an
    // adjustment line between them goes RED; no `let zoom` / `zoom =` rebind before the bind so
    // an upstream re-based zoom goes RED.
    let dem = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/world/terrain/relief/host.rs"
    )));
    let push = only_body(&dem, &format!("fn {}", "push_contours("));
    let bind = format!("let m_per_px = 2.0_f64.{}(-zoom);", "powf");
    let call = format!("{}(m_per_px)", "contour_interval_for_zoom");
    let bind_at = push
        .find(&bind)
        .expect("T-639/T-670/T-755: push_contours must bind m_per_px = 2^(-zoom)");
    let call_at = push
        .find(&call)
        .expect("T-639/T-670/T-755: push_contours must feed that m_per_px to the ladder");
    assert!(
        call_at > bind_at,
        "T-670/T-755: contour_interval_for_zoom(m_per_px) must follow the 2^(-zoom) bind"
    );
    let between = &push[bind_at + bind.len()..call_at];
    let squeezed: String = between.split_whitespace().collect::<Vec<_>>().join(" ");
    assert_eq!(
        squeezed, "let interval =",
        "T-670/T-755: bind and ladder call must be adjacent (`let interval =` only between);              got {squeezed:?}"
    );
    let before = &push[..bind_at];
    assert!(
        !before.contains("let zoom"),
        "T-670/T-755: push_contours must not rebind `zoom` before computing m_per_px"
    );
    let before_sq: String = before.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        !before_sq.contains("zoom ="),
        "T-670/T-755: push_contours must not assign `zoom` before computing m_per_px"
    );

    // (3) Numerically: our fn matches the shared convention across the clamp, and the printed
    // string stays within display precision of that same quantity.
    let mut z = MIN_ZOOM;
    while z <= MAX_ZOOM {
        let shared = 2.0_f64.powf(-z);
        assert!(
            (shared - m_per_px(z)).abs() < 1e-12,
            "zoom {z}: m_per_px must equal the shared 2^(-zoom) convention"
        );
        let shown: f64 = format_m_per_px(m_per_px(z))
            .trim_end_matches(" m/px")
            .parse()
            .expect("parseable readout");
        assert!(
            (shown - shared).abs() <= shared * 0.0051,
            "zoom {z}: the printed {shown} m/px must be the live {shared} m/px to display                  precision"
        );
        z += 0.125;
    }
}
/// **Reconciliation with T-667 (wave 106).** One scale, two surfaces: the graphic bar and this
/// number must be the same measurement. Given the same `m_per_px`, the bar's chosen ground
/// distance and the printed number are consistent — the bar is `dist_m / m_per_px` px long,
/// which is exactly what the printed number says it should be.
#[test]
fn the_bar_and_the_number_describe_the_same_scale() {
    let mut z = MIN_ZOOM;
    while z <= MAX_ZOOM {
        let mpp = m_per_px(z);
        let spec = pick_scale_bar(mpp);
        let shown: f64 = format_m_per_px(mpp)
            .trim_end_matches(" m/px")
            .parse()
            .expect("parseable readout");
        // Measuring the drawn bar with the printed scale recovers its labelled distance to
        // within the readout's own display precision (≤ 0.5%).
        let measured = spec.width_px * shown;
        assert!(
            (measured - spec.dist_m).abs() <= spec.dist_m * 0.0051,
            "zoom {z}: a {:.1} px bar read at {shown} m/px measures {measured} m, but is \
             labelled {} m — the two scale surfaces disagree",
            spec.width_px,
            spec.dist_m
        );
        z += 0.125;
    }
}

/// (wiring) The cell is a REAL fourth cell of the OBJ/SEL/SZ mono group in `StatusBar` — not a
/// floating span elsewhere in the bar — it carries a DOM handle and its own tooltip, and it
/// renders through the pure formatter above rather than an inline `format!`.
#[test]
fn the_scl_cell_sits_in_the_objselsz_group() {
    let src = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/toolbelt.rs"
    )));
    let status = only_body(&src, &format!("pub fn {}", "StatusBar("));
    let hook = format!("data-status-{}", "scale");
    let at = status
        .find(&hook)
        .expect("T-670: the scale readout must carry a DOM handle");
    // It is inside the SAME group div as OBJ/SEL/SZ: the SZ cell precedes it and the group's
    // closing </div> follows it, with no intervening element opening a new group.
    let sz = status
        .find(&["\"S", "Z\""].concat())
        .expect("SZ cell present");
    assert!(
        sz < at,
        "T-670: the scale cell must be the FOURTH cell — after SZ, inside the same group"
    );
    let group_end = status[sz..]
        .find("</div>")
        .map(|i| sz + i)
        .expect("the OBJ/SEL/SZ group closes");
    assert!(
        at < group_end,
        "T-670: the scale cell must close inside the OBJ/SEL/SZ group, not after it"
    );
    // Its own tooltip (the group title covers OBJ/SEL only), and the label the operator reads.
    let cell_end = status[at..]
        .find("</span>")
        .map(|i| at + i)
        .unwrap_or(status.len());
    let cell = &status[at..cell_end];
    assert!(
        cell.contains("title=") && cell.contains(&["S", "CL"].concat()),
        "T-670: the scale cell must carry its own title and the SCL label"
    );
    // The rendered value goes through the pure formatter (proven on scrubbed CODE, so a
    // mention in a comment or a class string cannot satisfy it).
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/toolbelt.rs"
    )));
    let status_code = only_body(&code, &format!("pub fn {}", "StatusBar("));
    assert!(
        status_code.contains(&format!("{}(", "format_m_per_px")),
        "T-670: the cell must render through format_m_per_px, not an inline format!"
    );
}

/// (single source) The status bar forwards its scale signal INTO the T-667 scale bar, and the
/// bar resolves from that signal when it has one — so the number and the graphic can never be
/// two independently-sampled zooms that disagree. The engine-less `camera_snapshot` fallback
/// survives for native/compat callers.
#[test]
fn the_scale_bar_resolves_from_the_same_signal() {
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/toolbelt.rs"
    )));
    let status = only_body(&code, &format!("pub fn {}", "StatusBar("));
    assert!(
        status.contains(&format!("{} cursor debug_hud scale_mpp", "<ScaleBar")),
        "T-670: StatusBar must forward scale_mpp into the ScaleBar (one scale, two surfaces)"
    );
    let bar = only_item(&code, &format!("pub fn {}", "ScaleBar("));
    assert!(
        bar.contains("scale_mpp: Option<RwSignal<f64>>"),
        "T-670: ScaleBar must accept the shared scale signal"
    );
    // It PREFERS the signal: the early return off `scale_mpp` precedes the camera re-read.
    let prefer = bar
        .find(&format!("{} = scale_mpp {{", "if let Some(s)"))
        .expect("T-670: ScaleBar must branch on the shared signal");
    let snapshot = bar
        .find(&format!("{}()", "camera_snapshot"))
        .expect("T-667: the camera fallback must survive for engine-less callers");
    assert!(
        prefer < snapshot,
        "T-670: the shared signal must take precedence over a second camera read"
    );
    // Wave 133 F2 / T-756 NIT-3 — comment corrections (seed / camera_snapshot-dead notes).
    // Raw include_str keeps docs that live_code blanks; only_item scopes to ScaleBar so the
    // test module cannot hollow-self-match; needles are fragment-assembled.
    let docs = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/toolbelt.rs"
    ));
    let bar_docs = only_item(docs, &format!("pub fn {}", "ScaleBar("));
    let seeded = format!("{}{}", "seeded ", "4.0");
    let cam_dead = format!("{}{}", "dead on the only real ", "caller");
    assert!(
        bar_docs.contains(&seeded),
        "T-756 / wave 133 F2: ScaleBar docs must keep the NIT-3 seed note (4 m/px default)"
    );
    assert!(
        bar_docs.contains(&cam_dead),
        "T-756 / wave 133 F2: ScaleBar docs must keep the NIT-3 camera_snapshot-dead note"
    );
}
