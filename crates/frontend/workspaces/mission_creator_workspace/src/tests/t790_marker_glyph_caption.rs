use super::marker_lane_fields;

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
/// alias→glyph mapping is asserted in `unit_symbology::markers`' own tests; here we prove the
/// ALIAS is carried so the mapper can see it.
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
