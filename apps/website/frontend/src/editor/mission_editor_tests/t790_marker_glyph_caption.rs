use super::marker_lane_fields;
use crate::editor::arsenal::class_r_scrub::{live_code, only_body};

/// Three markers of THREE different icons, one with a caption, one faction each — the acceptance
/// shape. Emitted in the `briefing_marker_rows_json` field vocabulary (x/z/factionId/icon/label).
fn rows() -> String {
    serde_json::json!([
        { "factionId": "faction-BLUFOR", "id": "m1", "x": 100.0, "z": 200.0,
          "icon": "attack",  "label": "Assault Bravo" },
        { "factionId": "faction-OPFOR",  "id": "m2", "x": 300.0, "z": 400.0,
          "icon": "defend",  "label": "" },
        { "factionId": "faction-INDFOR", "id": "m3", "x": 500.0, "z": 600.0,
          "icon": "flag",    "label": "Rally" },
    ])
    .to_string()
}

/// The authored `icon` alias and `label` caption both reach the lane arrays verbatim (the T-790
/// write-half: before this they were dropped), and the side tints follow the faction. The
/// alias→glyph mapping is asserted in `map_engine_render::scene`'s own tests (a wasm32-only dep
/// this native test cannot link); here we prove the ALIAS is carried so the mapper can see it.
#[test]
fn all_four_arrays_carry_the_authored_marker() {
    let (xy, tints, icons, captions) = marker_lane_fields(&rows());
    assert_eq!(xy, vec![100.0, 200.0, 300.0, 400.0, 500.0, 600.0]);
    assert_eq!(icons, vec!["attack", "defend", "flag"]);
    assert_eq!(captions, vec!["Assault Bravo", "", "Rally"]);
    // three different authored icons carried (so three distinct glyphs are reachable downstream)
    assert_eq!(
        icons.iter().collect::<std::collections::HashSet<_>>().len(),
        3,
        "three different icons must be carried distinctly"
    );
    // tints: BLUFOR / OPFOR / INDFOR (12 bytes), and they differ.
    assert_eq!(tints.len(), 12);
    assert_ne!(&tints[0..4], &tints[4..8], "BLUFOR vs OPFOR tint");
    assert_ne!(&tints[4..8], &tints[8..12], "OPFOR vs INDFOR tint");
}

/// Malformed / empty inputs are inert (no panic, four empty arrays) — the same shape the wasm
/// feed's early returns rely on.
#[test]
fn bad_input_is_inert() {
    for s in ["", "not json", "{}", "null", "[]"] {
        let (xy, t, g, c) = marker_lane_fields(s);
        assert!(
            xy.is_empty() && t.is_empty() && g.is_empty() && c.is_empty(),
            "{s:?}"
        );
    }
}

/// Class-R: the T-790 widening of the T-760 feed pin. Both `mission_history` feed sites must call
/// `markers_bind`, and the shared `marker_lane_xy_tints` builder they call must source ALL FOUR
/// arrays from the owned `marker_lane_fields` parse (so `icon`/`label` reach the lane). Deleting
/// the glyph or caption plumbing turns this RED; the map-engine-render lane-order pins never look
/// at `mission_history`, so this is the only guard that the write-half stays wired.
#[test]
fn both_feeds_pass_glyphs_and_captions() {
    let hist = live_code(include_str!("../state/history.rs"));
    let bind = format!("{}{}", "markers", "_bind");
    let fields = format!("{}{}", "marker_lane_", "fields");
    for site in ["pub fn rebind_engine_from_doc", "fn after_doc_change"] {
        let body = only_body(&hist, site);
        assert!(
            body.contains(&bind),
            "T-790: {site} must call markers_bind; body:\n{body}"
        );
    }
    // The single builder delegates to the owned (natively tested) parse — the icon + caption
    // arrays are read from the document exactly once, not re-derived per feed.
    let builder = only_body(&hist, "fn marker_lane_xy_tints");
    assert!(
        builder.contains(&fields),
        "T-790: marker_lane_xy_tints must source glyphs + captions from marker_lane_fields; \
         body:\n{builder}"
    );
}
