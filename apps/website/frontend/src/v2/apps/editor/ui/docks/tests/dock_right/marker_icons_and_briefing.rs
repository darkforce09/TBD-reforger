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
/// (which would mean two rows differing only by case slipped through). The GLYPH round-trip —
/// that each slug maps to a DISTINCT glyph via `scene::marker_glyph_for_alias` — cannot be
/// checked here (that mapper is a wasm32-only dep this native test cannot link); it is enforced
/// on the wasm side by the runtime `debug_assert` + self-healing fallback in
/// `canonical_marker_rows`, and the row COUNT is tied to `scene::MARKER_GLYPH_COUNT` by a
/// compile-time `const _` assert in the wasm build.
///
/// Source of truth for the count is `map_engine_render::scene::MARKER_GLYPH_COUNT`; the mirrored
/// [`CANONICAL_MARKER_GLYPH_COUNT`] carries a comment saying so and the wasm build asserts they
/// agree.
///
/// Perturbation RED: change any canonical slug to a non-enum value (e.g. `attack` → `assult`)
/// and the authorability loop fails; add a 12th slug that duplicates an existing family's label
/// and the case-collapse assertion fails.
#[test]
fn picker_has_one_row_per_canonical_icon() {
    use crate::v2::apps::editor::ui::inspector::zones_panel::humanize_token;

    // The documented row count — far below the 64 raw aliases (that shrink is the fix).
    assert_eq!(
        CANONICAL_MARKER_SLUGS.len(),
        CANONICAL_MARKER_GLYPH_COUNT,
        "the canonical slug table length is the picker row count"
    );
    assert_eq!(
        CANONICAL_MARKER_GLYPH_COUNT, 11,
        "the documented canonical glyph count (scene::MARKER_GLYPH_COUNT)"
    );
    assert!(
        CANONICAL_MARKER_GLYPH_COUNT < marker_icons().len(),
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

    // 'attack' — the acceptance's example pick — must be the canonical slug for its family, so
    // picking it stores `attack` (and the post-T-790 map draws the attack glyph). We assert the
    // STORED slug here; the glyph it maps to is the wasm sibling's job.
    assert!(
        CANONICAL_MARKER_SLUGS.contains(&"attack"),
        "'Attack' must store the canonical slug `attack`"
    );
}

/// **T-806 (F-08) — the picker ROW wiring: SVG glyph, canonical arm, slug demoted.**
///
/// Source-inspection (the render is `#[cfg(target_arch = "wasm32")]`, so a native test cannot
/// mount it): the picker must draw a real per-glyph SVG (not the generic `place` Material pin the
/// 64-row version used on every row), arm the canonical slug through `begin_place_marker`, and
/// build its rows through the canonical folder rather than the flat alias filter.
///
/// Haystacks are `class_r_scrub`-scrubbed so a needle satisfied by a comment or a blanked string
/// literal cannot pass; needles that are literals (`place`) are checked against the SCRUBBED-code
/// form where such literals are blanked, so only a live call satisfies them. Needles are
/// fragment-assembled where a contiguous spelling here would be its own decoy.
///
/// Perturbation RED: repoint the picker back to `filter_marker_icons` and re-add the `place` pin
/// per row — the `builds_rows` / `draws_svg` needles below fail; drop `marker_glyph_svg` and
/// the SVG presence check fails.
#[test]
fn picker_rows_draw_glyph_svgs_and_arm_the_canonical_slug() {
    use crate::v2::core::test_support::class_r_scrub::only_body;
    const SRC: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right.rs"
    ));

    // `markers_panel` has two definitions (the wasm picker + the native shell), so it is not a
    // unique `only_body` anchor. The picker rows live in the ONE `<ul aria-label="Marker icons">`
    // list, which precedes the `Authored markers` list; slice that block out and constrain it.
    // Fragment-assembled anchors so this test's own source is not the haystack.
    let icons_open = format!("aria-label={}Marker icons{}", '"', '"');
    let authored_open = format!("aria-label={}Authored markers{}", '"', '"');
    let icons_block = SRC
        .split(&icons_open)
        .nth(1)
        .and_then(|s| s.split(&authored_open).next())
        .expect("the picker's Marker icons list precedes the Authored markers list");

    // The picker builds one row per canonical glyph through the folder, not the flat filter, and
    // draws each row's glyph SVG via the per-glyph renderer.
    let builds_rows = format!("{}{}", "canonical_marker_rows", "(&icon_search.get())");
    assert!(
        SRC.contains(&builds_rows),
        "the picker must source its rows from the canonical folder"
    );
    let draws_svg = format!("{}{}", "marker_glyph_svg", "(glyph)");
    assert!(
        icons_block.contains(&draws_svg),
        "each picker row must render its canonical glyph SVG"
    );

    // Arming stores the canonical SLUG (the row's `armed`, built from `row.slug`).
    assert!(
        icons_block.contains(&format!("{}{}", "begin_place_marker", "(armed.clone())")),
        "picking a row must arm the marker place path with the canonical slug"
    );
    let arm_src = format!("let armed = {}", "row.slug.to_string();");
    assert!(
        icons_block.contains(&arm_src),
        "the armed value must be the canonical slug, not a raw alias"
    );

    // The old per-row generic `place` pin must be gone from the picker rows. It still exists in
    // the AUTHORED-markers list BELOW the picker (that list keeps its caption-first rows per the
    // trap), which is why the check is scoped to the icons block, not the whole file.
    assert!(
        !icons_block.contains(&format!("name={}place{}", '"', '"')),
        "the picker rows must not paint the generic `place` pin any more"
    );

    // The glyph renderer emits an <svg> element (a DOM shape), and the glyph vocabulary draws
    // distinct SVG primitives — the per-glyph DOM signature that differs from the old generic
    // pin. `marker_glyph_svg` is a unique anchor.
    let svg_fn = only_body(SRC, &format!("fn marker_glyph_svg{}", "("));
    assert!(
        svg_fn.contains("<svg"),
        "marker_glyph_svg must emit an <svg> element"
    );
    for prim in ["<polygon", "<circle", "<rect", "<path"] {
        assert!(
            svg_fn.contains(prim),
            "the glyph vocabulary must draw distinct primitives; missing {prim}"
        );
    }
}

/// **T-069 — markers are authored on the BRIEFING, never on the `markersById` root map.**
///
/// The ticket's own registry summary says free placement needs generic add/move/remove on
/// `markersById`. That premise is dead: `mission.schema.json` declares markers in exactly one
/// place (`$defs/briefing.markers[]`) and no top-level `markers` property at all, and
/// `flatten_to_mod_document` deserialises an `EditorPayload` that declares no root `markers`
/// key (it does declare zones/entities/vehicles/…) — so the root map is a closed hydrate→emit
/// loop and a marker authored there reaches no mod subsystem.
/// `store.rs`'s `a_marker_in_the_root_map_never_reaches_the_compiled_document` proves that end
/// of it; this pins that the PRODUCT surface never went to the dead one.
///
/// Every literal is split so this test's own source cannot satisfy the search it performs.
#[test]
fn marker_writes_go_to_the_briefing_not_the_root_map() {
    // T-934.7 — the ops module was split; the marker pins and the root-map absence scan
    // every submodule so the file-wide claims keep their whole-module meaning.
    let ops_all = [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/editing/hosted_commands/slot_attributes.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/arsenal/loadout_commands.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/editing/hosted_commands/slot_loadouts.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/editing/hosted_commands/composition_library.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/data/store/operations/compositions.rs"
        )),
        crate::v2::core::test_support::editor_operations::CONTEXT,
        crate::v2::core::test_support::editor_operations::ENTITY,
        crate::v2::core::test_support::editor_operations::DOMAIN_ENTITY,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/editing/hosted_commands/selection_transform.rs"
        )),
    ]
    .concat();
    #[allow(non_snake_case)]
    let OPS: &str = &ops_all;

    assert!(
        OPS.contains(&format!("core.{}_faction_briefing_marker(", "set")),
        "a marker place must write the faction briefing"
    );
    assert!(
        OPS.contains(&format!("core.{}_faction_briefing_marker(", "remove")),
        "a marker delete must go through the briefing mutator"
    );
    // No mutation of the root map anywhere in the ops surface.
    assert!(
        !OPS.contains(&format!("{}.insert(", "markers_root")),
        "the `markersById` root must stay unauthored"
    );
    // The vocabulary gate is on the write path, not merely on the picker.
    assert!(
        OPS.contains(&format!("marker_icon_{}(", "is_authorable")),
        "marker writes must gate on the closed icon enum"
    );
    // SCOPE GUARD — the authored row is the four schema-carried fields and its address, and
    // nothing else. `$defs/marker` also declares `size` / `rotationDeg` / `shape` / `area`, each
    // stamped "T-673, ships after T-069" in the schema itself; authoring any of them would
    // convert this from a factory ticket into a workbench one. Pinned on the STRUCT rather than
    // by grepping the field names, because those names legitimately appear in the prose that
    // explains why they are excluded — a token search over a file that documents its own
    // boundary finds the boundary.
    let row = OPS
        .split("pub struct MarkerRow {")
        .nth(1)
        .expect("MarkerRow is declared in editor_ops")
        .split("\n}")
        .next()
        .expect("MarkerRow has a body");
    let fields: Vec<&str> = row
        .lines()
        .filter_map(|l| l.trim().strip_prefix("pub "))
        .filter_map(|l| l.split(':').next())
        .collect();
    assert_eq!(
        fields,
        vec!["faction_id", "id", "x", "z", "icon", "label"],
        "MarkerRow is the four `$defs/marker` fields plus the (factionId, id) address"
    );
}

/// T-763 — Marker Attributes selects by `(factionId, id)`, never id alone.
///
/// A hydrated foreign payload can carry the same marker id under two factions; id-alone
/// `find` would edit the first match while the operator has the second selected. Needles are
/// fragment-assembled so this module is not its own haystack.
#[test]
fn marker_attributes_selects_by_faction_id_and_id() {
    const SRC: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/docks/dock_right.rs"
    ));
    let production = SRC
        .split("#[cfg(test)]")
        .next()
        .expect("eden_dock_right.rs must have a #[cfg(test)] module");
    // Scope to the Markers panel body — triggers still select by id alone, legally.
    let panel = production
        .split("pub(crate) fn markers_panel(")
        .nth(1)
        .and_then(|t| t.split("fn marker_attributes(").next())
        .expect("markers_panel must precede marker_attributes");
    let find = format!("r.faction_id == {} && r.id == {}", "faction_id", "id");
    assert!(
        panel.contains(&find),
        "Attributes lookup must match on (factionId, id); got no `{find}` in markers_panel"
    );
    assert!(
        !panel.contains(&format!("{}{}", "find(|r| r.id == ", "id)")),
        "markers_panel must not select by id alone"
    );
    assert!(
        production.contains(&format!("{}{}", "None::<(String, ", "String)>")),
        "marker_selected must be Option<(factionId, id)>"
    );
    // wave-136 F4 — pin selection addr construction at the click site. Pair-find alone greened
    // when the write used `(String::new(), m.id.clone())`.
    assert!(
        panel.contains("let addr = (m.faction_id.clone(), m.id.clone())"),
        "markers_panel click must build addr from both faction_id and id"
    );
}
