use super::*;

// ── T-069 (RIGHT-MODE-006) — the marker icon vocabulary ──────────────────────────────────

/// **T-069 — the icon list IS the schema's, alias for alias.**
///
/// `$defs/marker.icon` is a CLOSED enum, and the reason it is closed is a measured failure: a
/// typo or empty string used to validate clean and then DEGRADE at runtime, `Resolve()`
/// returning the fallback DOT glyph and logging once — the marker drew, but not as authored. A
/// hand-copied `const MARKER_ICONS: [&str; 64]` in the dock would reopen that hole the first
/// time the schema moved, so the list is PARSED from the embedded schema and this test re-reads
/// the same bytes independently and compares in order.
///
/// Perturbation RED: dropping any alias from the parse (or hard-coding the list) fails the
/// element-wise comparison naming the index.
#[test]
fn the_icon_list_is_the_schemas_own() {
    let schema: serde_json::Value =
        serde_json::from_str(MISSION_SCHEMA_JSON).expect("the embedded schema must parse");
    let expected: Vec<&str> = schema["$defs"]["marker"]["properties"]["icon"]["enum"]
        .as_array()
        .expect("$defs/marker.icon declares an enum")
        .iter()
        .map(|v| v.as_str().expect("every alias is a string"))
        .collect();

    let got: Vec<&str> = marker_icons().iter().map(String::as_str).collect();
    assert_eq!(got, expected, "the panel's list must be the schema's list");
    assert_eq!(
        expected.len(),
        64,
        "the enum is closed at 64 aliases; a change here is a schema widening, which T-069 \
         is explicitly not"
    );

    // The vocabulary is not the marker SHAPE vocabulary — `shape` (T-673, ships after this) is
    // a different, four-value enum, and picking it up here would author style this slice does
    // not own.
    assert!(
        !got.contains(&"rectangle") && !got.contains(&"polyline"),
        "`$defs/marker.shape` values must not leak into the icon list: {got:?}"
    );
}

/// **T-069 — an alias outside the closed enum is refused, and the refusal is case-sensitive.**
///
/// Every marker write in `editor_ops` gates on this predicate, so it is the whole enforcement.
/// `hazard` is the pointed case: `store.rs`'s own T-345 tests author it, because the store
/// mutator takes an `&str` and asks no questions — deliberately, so those pins stay green. The
/// vocabulary is enforced at the PRODUCT boundary, which is here.
#[test]
fn only_schema_aliases_are_authorable() {
    assert!(marker_icon_is_authorable("dot"));
    assert!(marker_icon_is_authorable("objective"));
    assert!(marker_icon_is_authorable("rally_point"));

    assert!(!marker_icon_is_authorable(""), "empty is not an alias");
    assert!(
        !marker_icon_is_authorable("hazard"),
        "`hazard` is not in the enum, however plausible it reads"
    );
    assert!(
        !marker_icon_is_authorable("Objective"),
        "the enum is lower-case and validators do not case-fold"
    );
    assert!(
        !marker_icon_is_authorable("dot "),
        "no trimming, no guessing"
    );

    // The default a fresh place uses is itself an authorable alias, not a literal that could
    // drift out of the enum.
    assert!(
        marker_icon_is_authorable(default_marker_icon()),
        "the default icon must be in the closed list: {:?}",
        default_marker_icon()
    );
}

/// **T-069 — the icon search filters the closed list and can never widen it.**
///
/// An empty query lists everything (RIGHT-MODE-006's "Marker icons in list"), and every result
/// of every query is an alias the schema declares — the filter narrows, it never invents.
#[test]
fn the_icon_search_narrows_the_closed_list() {
    assert_eq!(
        filter_marker_icons("").len(),
        marker_icons().len(),
        "an empty query lists every icon"
    );
    assert_eq!(filter_marker_icons("   ").len(), marker_icons().len());

    let obj = filter_marker_icons("objective");
    assert!(obj.contains(&"objective"), "{obj:?}");
    assert!(obj.contains(&"objective_marker"), "{obj:?}");
    assert!(!obj.contains(&"dot"), "{obj:?}");

    // Case-insensitive, and `_` reads as a space so a typed phrase finds the token.
    assert!(filter_marker_icons("RALLY").contains(&"rally_point"));
    assert!(filter_marker_icons("rally point").contains(&"rally_point"));

    assert!(
        filter_marker_icons("zzz-not-an-icon").is_empty(),
        "a miss is empty, not a fallback"
    );

    for q in ["", "a", "point", "OBS", "medic"] {
        for hit in filter_marker_icons(q) {
            assert!(
                marker_icon_is_authorable(hit),
                "the filter may only return schema aliases; {q:?} yielded {hit:?}"
            );
        }
    }
}

/// **T-806 (F-08) — the picker is one row per CANONICAL icon, not 64 raw slugs.**
///
/// The defect was 64 rows of raw aliases, all wearing the same generic pin, with case-duplicates
/// (Waypoint/waypoint, Objective/Obj/Target, mark/marker/point) shown as separate rows. The fix
/// collapses the display onto T-790's glyph families. This test pins the NATIVE half of that
/// contract — the canonical slug set: its count is the documented < 64 number, every slug is a
/// closed-enum member (a pick validates and saves), and no two rows collapse to the same label
/// (which would mean two rows differing only by case slipped through). It also checks the glyph
/// round-trip against the source of truth: the picker's row count is
/// `unit_symbology::markers::MARKER_GLYPH_COUNT`, each slug folds to a DISTINCT glyph through
/// `unit_symbology::markers::marker_glyph_for_alias`, and the rows the panel builds number
/// exactly that many glyphs.
///
/// Perturbation RED: change any canonical slug to a non-enum value (e.g. `attack` → `assult`)
/// and the authorability loop fails; add a 12th slug that duplicates an existing family's label
/// and the case-collapse assertion fails.
#[test]
fn picker_has_one_row_per_canonical_icon() {
    use mission_creator_state::marker_icons::canonical_marker_rows;
    use mission_creator_state::zones::humanize_token;
    use unit_symbology::markers::{MARKER_GLYPH_COUNT, marker_glyph_for_alias};

    // The documented row count — far below the 64 raw aliases (that shrink is the fix).
    assert_eq!(
        CANONICAL_MARKER_SLUGS.len(),
        MARKER_GLYPH_COUNT,
        "the canonical slug table length is the map's glyph count"
    );
    assert_eq!(
        MARKER_GLYPH_COUNT, 11,
        "the documented canonical glyph count (unit_symbology::markers::MARKER_GLYPH_COUNT)"
    );
    assert!(
        MARKER_GLYPH_COUNT < marker_icons().len(),
        "the picker must have FEWER rows than the {} raw aliases — that collapse is F-08",
        marker_icons().len()
    );

    // Every canonical slug is a member of the closed enum: a pick validates and saves.
    for slug in CANONICAL_MARKER_SLUGS {
        assert!(
            marker_icon_is_authorable(slug),
            "canonical slug {slug:?} must be a closed `$defs/marker.icon` enum member"
        );
    }

    // No two rows share a label: a duplicate would be exactly the "two rows differ only by case"
    // defect surviving. Labels are what the row shows (`humanize_token` of the slug).
    let mut labels: Vec<String> = CANONICAL_MARKER_SLUGS
        .iter()
        .map(|s| humanize_token(s).to_ascii_lowercase())
        .collect();
    labels.sort();
    let unique = labels
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len();
    assert_eq!(
        unique,
        labels.len(),
        "no two canonical rows may share a (case-folded) label: {labels:?}"
    );

    // The canonical slugs are themselves distinct strings (no accidental repeat in the table).
    let slug_set = CANONICAL_MARKER_SLUGS
        .iter()
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        slug_set.len(),
        CANONICAL_MARKER_SLUGS.len(),
        "canonical slug table has a duplicate entry"
    );

    // Every canonical slug folds to its own glyph, and the panel builds one row per glyph: the
    // display and the map agree on the family set.
    let glyphs = CANONICAL_MARKER_SLUGS
        .iter()
        .map(|slug| marker_glyph_for_alias(slug) as usize)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        glyphs.len(),
        MARKER_GLYPH_COUNT,
        "every canonical slug must fold to a distinct map glyph: {CANONICAL_MARKER_SLUGS:?}"
    );
    assert_eq!(
        canonical_marker_rows("").len(),
        MARKER_GLYPH_COUNT,
        "the picker builds one row per map glyph"
    );

    // 'attack' — the acceptance's example pick — must be the canonical slug for its family, so
    // picking it stores `attack`; the distinct-glyph check above covers the glyph it draws.
    assert!(
        CANONICAL_MARKER_SLUGS.contains(&"attack"),
        "'Attack' must store the canonical slug `attack`"
    );
}
