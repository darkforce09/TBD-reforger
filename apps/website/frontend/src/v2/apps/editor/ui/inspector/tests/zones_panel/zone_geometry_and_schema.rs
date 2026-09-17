use super::{
    circle_from_clicks, humanize_key, humanize_token, polygon_flat, polygon_is_committable,
    radius_survives_compile, round_coord, terrain_rect_is_authorable, terrain_rect_ring,
    whole_terrain_zone_type, zone_rule_fields, zone_types, ZoneRuleKind, MIN_AUTHORABLE_RADIUS_M,
    MISSION_SCHEMA, WHOLE_TERRAIN_ZONE_LABEL, ZONE_GRID_M,
};

/// The quantisation this file mirrors is `flatten::round_coord`, which is private there. Pin it
/// against that source so the mirror cannot drift silently — RED if `flatten.rs` changes its
/// grid without this file following. Same guard `api/src/contract/validate.rs` puts on its own
/// copy of the same line.
#[test]
fn zone_quantisation_mirrors_flatten() {
    let flatten = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/data/scenario/compiler/flatten/zones.rs"
    ));
    let body = flatten
        .split("fn round_coord(v: f64) -> f64 {")
        .nth(1)
        .expect("flatten::round_coord must exist");
    let expr = body.split('}').next().expect("body").trim();
    assert_eq!(
        expr, "(v * 10.0).round() / 10.0",
        "flatten::round_coord changed — update eden_chrome::round_coord to match"
    );
    // And the mirror agrees on the value that produced the defect T-581 documented.
    assert_eq!(round_coord(0.04), 0.0);
    assert_eq!(round_coord(0.05), 0.1);
}

/// The published minimum is a CONSEQUENCE of the grid, not an independent constant. If the grid
/// ever changes, this fails rather than letting the UI advertise a stale threshold.
#[test]
fn min_radius_is_the_grid_consequence() {
    assert!(
        radius_survives_compile(MIN_AUTHORABLE_RADIUS_M),
        "the advertised minimum must itself survive the compile"
    );
    assert!(
        !radius_survives_compile(MIN_AUTHORABLE_RADIUS_M - ZONE_GRID_M / 100.0),
        "anything below the advertised minimum must be refused"
    );
    // The exact radius a click-without-travel produced before this tool existed (T-581).
    assert!(!radius_survives_compile(0.04));
    assert!(!radius_survives_compile(0.0));
    assert!(!radius_survives_compile(-5.0));
    assert!(!radius_survives_compile(f64::NAN));
    assert!(radius_survives_compile(250.0));
}

/// A click without travel is the r=0.04 shape T-581 has to reject at save. The tool refuses to
/// CREATE it, so the save-time check is a backstop rather than the first line of defence.
#[test]
fn circle_refuses_the_click_without_drag() {
    assert_eq!(circle_from_clicks(100.0, 200.0, 100.0, 200.0), None);
    // 0.04 m of travel — schema-valid authored, schema-INVALID once quantised.
    assert_eq!(circle_from_clicks(0.0, 0.0, 0.04, 0.0), None);
    // A real drag survives, and carries the document's (x, z, r) — not the viewport's y.
    let (x, z, r) = circle_from_clicks(10.0, 20.0, 13.0, 24.0).expect("a real drag commits");
    assert!((x - 10.0).abs() < f64::EPSILON && (z - 20.0).abs() < f64::EPSILON);
    assert!((r - 5.0).abs() < 1e-12, "3-4-5 triangle: r = 5, got {r}");
    // NaN from a singular-matrix unproject must never reach the document.
    assert_eq!(circle_from_clicks(f64::NAN, 0.0, 1.0, 1.0), None);
}

/// `$defs/polygon` is `minItems: 3` and the doc layer deliberately does not guard it — its own
/// comment assigns the guard to this tool. Two vertices must not be committable.
#[test]
fn polygon_commits_only_at_three_vertices() {
    assert!(!polygon_is_committable(&[]));
    assert!(!polygon_is_committable(&[(0.0, 0.0)]));
    assert!(!polygon_is_committable(&[(0.0, 0.0), (10.0, 0.0)]));
    assert!(polygon_is_committable(&[
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0)
    ]));
    assert!(!polygon_is_committable(&[
        (0.0, 0.0),
        (10.0, 0.0),
        (f64::NAN, 10.0)
    ]));
    assert_eq!(
        polygon_flat(&[(1.0, 2.0), (3.0, 4.0), (5.0, 6.0)]),
        vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        "the doc layer takes a FLAT ring"
    );
}

/// ═══ THE ANTI-SECOND-VOCABULARY TEST ═══
///
/// The panel must render whatever `$defs/zoneRules` declares. This reads the vocabulary a SECOND
/// way — straight out of the embedded JSON — and demands the two agree key-for-key. A panel
/// built from a hand-typed list would pass every "the panel renders" assertion while silently
/// omitting a key; this is the assertion that cannot.
#[test]
fn zone_rule_fields_cover_the_whole_vocabulary() {
    let schema: serde_json::Value =
        serde_json::from_str(MISSION_SCHEMA).expect("mission.schema.json parses");
    let props = schema["$defs"]["zoneRules"]["properties"]
        .as_object()
        .expect("$defs/zoneRules/properties");
    assert!(
        !props.is_empty(),
        "an empty vocabulary would make the panel vacuously correct"
    );
    assert_eq!(
        schema["$defs"]["zoneRules"]["additionalProperties"],
        serde_json::Value::Bool(false),
        "the vocabulary must stay CLOSED — an open one would make this whole approach unsound"
    );

    let fields = zone_rule_fields();
    let mut from_fields: Vec<&str> = fields.iter().map(|f| f.key.as_str()).collect();
    let mut from_schema: Vec<&str> = props.keys().map(String::as_str).collect();
    from_fields.sort_unstable();
    from_schema.sort_unstable();
    assert_eq!(
        from_fields, from_schema,
        "every declared rule key must reach the panel, and the panel must invent none"
    );

    // A control per declared kind, so a new key of an existing shape needs no code here.
    let kind = |k: &str| {
        fields
            .iter()
            .find(|f| f.key == k)
            .unwrap_or_else(|| panic!("{k} missing"))
            .kind
            .clone()
    };
    assert!(matches!(
        kind("contestable"),
        ZoneRuleKind::Bool { default: true }
    ));
    match kind("penalty") {
        ZoneRuleKind::Choice { options, default } => {
            assert_eq!(options, vec!["none", "warn", "kill"]);
            assert_eq!(default.as_deref(), Some("warn"));
        }
        other => panic!("penalty must be a Choice, got {other:?}"),
    }
    match kind("graceSeconds") {
        ZoneRuleKind::Number {
            default,
            minimum,
            maximum,
            integer,
            ..
        } => {
            assert_eq!(default, Some(30.0));
            assert_eq!(minimum, Some(0.0));
            // T-275 pinned this to TBD_ZoneRegistry.MAX_GRACE_SECONDS.
            assert_eq!(maximum, Some(3600.0));
            assert!(!integer);
        }
        other => panic!("graceSeconds must be a Number, got {other:?}"),
    }
    match kind("warnEverySeconds") {
        ZoneRuleKind::Number {
            exclusive_minimum, ..
        } => assert_eq!(
            exclusive_minimum,
            Some(0.0),
            "0 would mean 'warn every frame' — the reader requires > 0"
        ),
        other => panic!("warnEverySeconds must be a Number, got {other:?}"),
    }
    assert!(
        matches!(
            kind("targetCount"),
            ZoneRuleKind::Number { integer: true, .. }
        ),
        "targetCount is the one integer"
    );
    // `targetAlias` is declared as a `$ref` — a resolver that ignored it would drop the pattern.
    match kind("targetAlias") {
        ZoneRuleKind::Text { pattern, .. } => assert_eq!(
            pattern.as_deref(),
            Some("^(kit|comp|veh|preset|layer|prop|item):[a-z0-9_]+$"),
            "the $ref into $defs/alias must be resolved"
        ),
        other => panic!("targetAlias must resolve to Text, got {other:?}"),
    }
    // Every field carries the schema's prose, which names the mod call site.
    assert!(
        fields.iter().all(|f| !f.doc.is_empty()),
        "each control shows the schema's own description"
    );
}

/// T-685 -- the six zone-volume keys must already render via `zone_rule_fields` (schema-
/// generated). This pins their KINDS so a generator that dropped `$ref` / integer / bounds
/// would still cover the vocabulary names and ship the wrong controls. Do not invent a second
/// inspector: a missing key fails `zone_rule_fields_cover_the_whole_vocabulary` first.
#[test]
fn t685_volume_fields_render_from_zone_rules_schema() {
    let fields = zone_rule_fields();
    let kind = |k: &str| {
        fields
            .iter()
            .find(|f| f.key == k)
            .unwrap_or_else(|| panic!("T-685: {k} missing from zone_rule_fields"))
            .kind
            .clone()
    };

    match kind("attackerCount") {
        ZoneRuleKind::Number {
            integer, minimum, ..
        } => {
            assert!(integer, "attackerCount is schema integer");
            assert_eq!(minimum, Some(0.0));
        }
        other => panic!("attackerCount must be Number, got {other:?}"),
    }
    match kind("defenderCount") {
        ZoneRuleKind::Number {
            integer, minimum, ..
        } => {
            assert!(integer, "defenderCount is schema integer");
            assert_eq!(minimum, Some(0.0));
        }
        other => panic!("defenderCount must be Number, got {other:?}"),
    }
    match kind("advantagePercent") {
        ZoneRuleKind::Number {
            integer,
            minimum,
            maximum,
            ..
        } => {
            assert!(!integer, "advantagePercent is schema number");
            assert_eq!(minimum, Some(0.0));
            assert_eq!(maximum, Some(100.0));
        }
        other => panic!("advantagePercent must be Number, got {other:?}"),
    }
    match kind("minHeight") {
        ZoneRuleKind::Number { integer, .. } => {
            assert!(!integer, "minHeight is schema number (AGL metres)");
        }
        other => panic!("minHeight must be Number, got {other:?}"),
    }
    match kind("maxHeight") {
        ZoneRuleKind::Number { integer, .. } => {
            assert!(!integer, "maxHeight is schema number (AGL metres)");
        }
        other => panic!("maxHeight must be Number, got {other:?}"),
    }
    match kind("startingOwner") {
        ZoneRuleKind::Text { pattern, .. } => assert_eq!(
            pattern.as_deref(),
            Some("^[a-z][a-z0-9_]*$"),
            "startingOwner must resolve $ref factionKey (side picker, not a free string)"
        ),
        other => panic!("startingOwner must resolve to Text, got {other:?}"),
    }
}

/// The type picker is schema-driven for the same reason the rules are: `set_zone_type` writes
/// whatever it is handed, and an invented seventh value saves 201 then 500s `/compiled`.
#[test]
fn zone_types_come_from_the_schema() {
    let schema: serde_json::Value =
        serde_json::from_str(MISSION_SCHEMA).expect("mission.schema.json parses");
    let declared: Vec<String> = schema["$defs"]["zone"]["properties"]["type"]["enum"]
        .as_array()
        .expect("$defs/zone/properties/type/enum")
        .iter()
        .map(|v| v.as_str().expect("string").to_string())
        .collect();
    assert_eq!(zone_types(), declared);
    assert!(
        zone_types().contains(&"boundary".to_string()),
        "the play-area type must be offerable"
    );
}

/// ═══ THE TICKET, AS AN ASSERTION ═══
///
/// T-582's measurement was "zero references to any zone mutator in the frontend — zones are
/// authorable only from native test code". This is that measurement, inverted and kept: every
/// one of T-211's eleven mutators must have a caller.
///
/// Source inspection, following `vehicles_tab_places_instead_of_promising`'s precedent, because
/// the thing under test is a wasm-only module (`editor_ops` is `#![cfg(target_arch = "wasm32")]`)
/// that no native test can link. A behavioural test is impossible here; a source assertion is
/// not, and the alternative is no assertion at all.
#[test]
fn every_t211_mutator_has_a_caller() {
    let ops = [
        crate::v2::core::test_support::editor_operations::ENTITY,
        crate::v2::core::test_support::editor_operations::DOMAIN_ENTITY,
    ]
    .concat();
    #[allow(non_snake_case)]
    let OPS: &str = &ops;
    // The eleven, verbatim from `doc/store.rs`'s T-211 block.
    for m in [
        "add_circle_zone",
        "add_polygon_zone",
        "set_zone_circle",
        "set_zone_polygon",
        "set_zone_type",
        "set_zone_label",
        "set_zone_faction",
        "set_zone_rules",
        "remove_zone",
        "zones_json",
        "zone_count",
    ] {
        assert!(
            OPS.contains(&format!("core.{m}(")) || OPS.contains(&format!("MissionDocCore::{m}")),
            "T-211 mutator `{m}` still has no caller — that was the whole T-582 defect"
        );
    }
    // And the reshape pair specifically: they are the two that are easy to leave unwired,
    // because create-only looks finished.
    assert!(
        OPS.contains("begin_zone_reshape"),
        "reshape must be reachable, not just create"
    );
}

/// Labels are presentation only — the token itself is what reaches the document.
#[test]
fn labels_never_replace_tokens() {
    assert_eq!(
        humanize_token("objective_hold_until"),
        "Objective hold until"
    );
    assert_eq!(humanize_token("spawn"), "Spawn");
    assert_eq!(humanize_key("warnEverySeconds"), "Warn every seconds");
    assert_eq!(humanize_key("penalty"), "Penalty");
    // Round-trip safety: a label is never fed back as a key.
    for f in zone_rule_fields() {
        assert_ne!(
            humanize_key(&f.key),
            f.key,
            "label must differ from wire key"
        );
    }
}

// ── T-792 — Esc cancels an in-progress zone (and trigger) draw ──────────────────────────────
// These are source pins because `editor_ops` / the `mission_editor` keydown closure are
// wasm-only (`#![cfg(target_arch = "wasm32")]`), the same reason `every_t211_mutator_has_a_caller`
// above is a source assertion — a behavioural test cannot link the module. All pins run on
// `class_r_scrub::live_code`, which DELETES comments and BLANKS string literals, so a needle can
// only be satisfied by real shipping code, never by the very comments that describe the fix (this
// is the T-759-class discipline: never grep the raw file for the token you just added).

/// The keydown Escape arm lives in the editor keydown dispatch — `input/window_keydown.rs` since
/// T-934.14 (it was inside `MissionEditorPage` before). Slice the page from its anchor before
/// scrubbing — the same anchor `t642_ruler_wiring::editor_live` uses (a whole-file `live_code`
/// prunes reachable-only-after-a-jump statements too aggressively for a deep-nested match arm)
/// — then append the commands file, scrubbed separately (the T-934.13 concat idiom).
/// `live_code` deletes comments + blanks string literals.
fn editor_live_from_page() -> String {
    use crate::v2::core::test_support::class_r_scrub::live_code;
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let raw = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/mission_editor.rs"
    ));
    assert_eq!(
        raw.matches(anchor.as_str()).count(),
        1,
        "scrub anchor must be unambiguous"
    );
    let mut src = live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..]);
    src.push_str(&live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/input/window_keydown.rs"
    ))));
    src
}

/// The keyboard Esc arm must ROUTE an in-progress draw to `cancel_zone_draw`. Before T-792 the arm
/// only called `cancel_pending`, which a zone draw deliberately survives — so F-31 was: arm Circle,
/// click the centre, press Esc, and the draft stayed armed (the panel kept prompting for the rim,
/// and the next click completed the circle). The pin proves the real call is present in the live
/// keydown closure, not in a comment.
#[test]
fn t792_escape_arm_cancels_the_zone_draw() {
    let ed = editor_live_from_page();
    assert!(
            ed.contains("armed_placement::cancel_zone_draw()"),
            "T-792: the editor keydown must call armed_placement::cancel_zone_draw() (the ONE cancel a \
             multi-click draw honours) — cancel_pending alone leaves the draft armed"
        );
    // It rides the SAME shared keydown Escape seam as the place/connect/measure cancels — not a
    // new window listener (the T-726 pile-up must not grow). Proven by co-location with the
    // keydown dispatch and the sibling place cancel it sits beside.
    assert!(
        ed.contains("code().as_str()")
            && ed.contains("armed_placement::has_pending()")
            && ed.contains("armed_placement::cancel_zone_draw()"),
        "T-792: the zone-draw cancel must live in the ONE shared keydown Escape arm, beside the \
             armed-place (has_pending) cancel — no second window keydown listener"
    );
}

/// One-Esc-one-layer (T-813/T-814): when the draw cancel ACTS it must feed the arm's "handled"
/// result, so the press is consumed (prevent_default) and no lower Esc layer — a dialog, a menu,
/// the tab — also closes on the SAME keypress. The binding `zone_draw_acted` must therefore flow
/// into the trailing `||` chain that the arm returns.
#[test]
fn t792_zone_cancel_consumes_the_press() {
    let ed = editor_live_from_page();
    assert!(
        ed.contains("let zone_draw_acted =") && ed.contains("|| zone_draw_acted"),
        "T-792: the draw-cancel result must join the Escape arm's handled `||` chain, so a real \
             cancel consumes the press (one Esc, one layer)"
    );
}

/// The hint-clear half. `cancel_zone_draw` must, on a real clear, bump the dock tick — the Zones
/// AND Triggers panels re-read `zone_draft()` under `doc_tick` and their "click the rim"/vertex
/// hint (plus the Cancel/Close controls) vanish once the draft is `None`. Mirrors T-791's
/// `cancel_pending` bump. Pinned on the function's OWN body via `only_body` (unique name), so a
/// bump elsewhere in the file cannot satisfy it.
#[test]
fn t792_cancel_zone_draw_bumps_the_dock_tick() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    let ops = live_code(crate::v2::core::test_support::editor_operations::ENTITY);
    let body = only_body(&ops, "pub fn cancel_zone_draw() -> bool");
    assert!(
        body.contains("bump_doc_tick()"),
        "T-792: cancel_zone_draw must bump the dock tick on a real clear, so the rim/vertex hint \
             (gated on zone_draft() under doc_tick) disappears — the panel-Cancel effect, on Esc"
    );
    // Collection-agnostic: it clears on `Pending::Zone(_)`, so the SHARED draft cancels a TRIGGER
    // draw exactly as it cancels a zone draw (trigger arms via begin_zone_draw(.., Trigger) into
    // the identical Pending::Zone). This is what lets ONE Esc call cover both consumers.
    assert!(
        body.contains("Some(Pending::Zone(_))"),
        "T-792: cancel_zone_draw must clear on Pending::Zone(_) regardless of collection, so a \
             trigger draw (the second consumer) is cancelled by the same call"
    );
}

/* ═════════════ T-702 (3DEN-MISC-001 E11) — the whole-terrain Play Area zone ═════════════ */

/// **BOTH shipped terrains.** `everon` (12800²) and `arland` (4096²) are the two the create
/// dialog offers (`library/create_dialog.rs`) and the only two `compile::terrain_bounds`
/// distinguishes; every other key resolves to Everon, which the golden below also covers by
/// running the unknown ones through the SAME comparison rather than assuming the fallback.
const SHIPPED_TERRAINS: [&str; 2] = ["everon", "arland"];

/// A compilable mission with `zones` set to `extra` — factions/squads/slots are present because
/// `flatten_to_mod_document` answers `CompileError::NoSlots` on a bare document, which would
/// fail this test for a reason with nothing to do with zones. Mirrors flatten's own
/// `zones_test_payload` fixture.
fn terrain_payload(extra: &str) -> Vec<u8> {
    format!(
        r#"{{
              "zones": {extra},
              "editor": {{
                "factions": [{{"key": "BLUFOR", "name": "US", "squadIds": ["sq_a"]}}],
                "squads": [{{"id": "sq_a", "callsign": "Alpha", "slotIds": ["s_a"]}}],
                "slots": [
                  {{"id": "s_a", "index": 0, "role": "RFL",
                   "position": {{"x": 1000.0, "y": 2000.0, "z": 0, "rotation": 0}}}}
                ]
              }}
            }}"#
    )
    .into_bytes()
}

/// Compile `payload` for `terrain` and hand back the mod document.
fn compile_for(
    terrain: &str,
    payload: &[u8],
) -> website_map_engine::data::scenario::flatten::ModMissionDocument {
    use website_map_engine::data::scenario::flatten::flatten_to_mod_document;
    use website_map_engine::data::scenario::flatten::MissionMeta;
    let meta = MissionMeta {
        terrain: terrain.to_string(),
        ..MissionMeta::default()
    };
    let doc = flatten_to_mod_document(&meta, payload)
        .unwrap_or_else(|e| panic!("{terrain} must compile: {e:?}"));
    // T-702 — a golden that only checks the SHAPE of a document has not checked WHICH document
    // it read. `flatten` latches `schemaVersion` on the payload's own keys (`1.1` / `1.2` /
    // `1.3`, flatten.rs), and this fixture carries none of the later ones, so `1.1` is both the
    // version under test and a statement of which payload shape produced it. A latch change
    // re-opens this comparison instead of letting it pass over a document whose `zones` no
    // longer mean what they meant here.
    assert_eq!(
        doc.schema_version, "1.1",
        "the mod-document format this golden was written against"
    );
    doc
}

/// The `boundary` zone the compile produced, as one flat `[x0,z0,…]` ring.
fn compiled_boundary_ring(
    doc: &website_map_engine::data::scenario::flatten::ModMissionDocument,
    id: &str,
) -> Vec<f64> {
    use website_map_engine::data::scenario::flatten::ModZoneShape;
    let z = doc
        .zones
        .iter()
        .find(|z| z.id == id)
        .unwrap_or_else(|| panic!("no zone `{id}` in the compiled document"));
    assert_eq!(z.kind, "boundary", "`{id}` must compile as the play area");
    let ModZoneShape::Polygon { polygon } = &z.shape else {
        panic!("`{id}` must be a POLYGON — a square terrain is not a disc");
    };
    polygon.iter().flat_map(|p| [p[0], p[1]]).collect()
}

/// ═══ THE GOLDEN: THE AUTHORED RECT **IS** THE COMPILED RECT ═══
///
/// The affordance's whole promise is "one zone sized to the map", and the only way to check that
/// without re-typing a number this file would then be the sole author of is to ask the COMPILE
/// what the map is. `flatten` already answers it: with no authored boundary it synthesises
/// `z_bounds` from `compile::terrain_bounds` (`flatten.rs` `synthesize_terrain_boundary`), which
/// is the same helper `validate`'s `V3-SLOT-IN-BOUNDS` and the editor's own `terrain_bounds_of`
/// read. So this compiles a zone-free mission for each terrain, takes the ring the compile
/// itself produced, and demands [`terrain_rect_ring`] equals it — coordinate for coordinate,
/// in the same winding.
///
/// That is what makes "matching terrain_bounds exactly" mechanical rather than aspirational: a
/// literal `12800` typed here would drift the day a terrain's extent changes; this cannot,
/// because both sides are the same source.
#[test]
fn whole_terrain_ring_is_the_rect_the_compile_reads() {
    use website_map_engine::data::scenario::compile::terrain_bounds;
    for terrain in SHIPPED_TERRAINS {
        let compiled =
            compiled_boundary_ring(&compile_for(terrain, &terrain_payload("[]")), "z_bounds");
        let authored = terrain_rect_ring(terrain, terrain_bounds(terrain))
            .unwrap_or_else(|| panic!("{terrain}'s own bounds must be authorable"));
        assert_eq!(
            authored, compiled,
            "the {terrain} whole-terrain ring must be the rect the compile reads, exactly"
        );
    }
    // The two terrains really are DIFFERENT rects — without this the loop above would pass just
    // as happily if `terrain_bounds` had collapsed to one size and the golden had followed it.
    assert_ne!(
        terrain_rect_ring("everon", terrain_bounds("everon")),
        terrain_rect_ring("arland", terrain_bounds("arland")),
        "everon 12800² and arland 4096² must not author the same ring"
    );
    // And an unknown terrain resolves to Everon's rect through the SAME path, rather than to
    // nothing — `terrain_bounds` documents that fallback and the affordance inherits it.
    for unknown in ["custom", "definitely-not-a-terrain", ""] {
        assert_eq!(
            terrain_rect_ring(unknown, terrain_bounds(unknown)),
            terrain_rect_ring("everon", terrain_bounds("everon")),
            "an unknown terrain must author Everon's rect, the fallback terrain_bounds takes"
        );
    }
}

/// ═══ AND IT REPLACES THE SYNTHESISED AO RATHER THAN SITTING BESIDE IT ═══
///
/// `flatten`'s `derive_zones` only synthesises `z_bounds` when the payload declares NO
/// `boundary` zone (`zones_have_boundary`). So a mission carrying the authored Play Area must
/// compile to exactly ONE boundary — the authored one, carrying the author's label — and the
/// synthesised fallback must be gone. If the two rings ever differed, pressing the button would
/// silently change the play area's edge from what the mission already compiled to; they do not,
/// and this is where that is checked end to end.
#[test]
fn the_authored_play_area_becomes_the_compiled_play_area() {
    use website_map_engine::data::scenario::compile::terrain_bounds;
    for terrain in SHIPPED_TERRAINS {
        // The play area the compile would have synthesised for this map, taken FROM the compile.
        // Building the payload out of `terrain_rect_ring` and then reading it back would make
        // this test self-consistent — measured: it stayed GREEN while `terrain_rect_corners`
        // was perturbed to a hardcoded 12800 rect, because it round-tripped the wrong rect just
        // as faithfully. So the fallback ring is the oracle and the authored ring is checked
        // against it before either reaches a payload.
        let synthesised =
            compiled_boundary_ring(&compile_for(terrain, &terrain_payload("[]")), "z_bounds");
        let ring = terrain_rect_ring(terrain, terrain_bounds(terrain)).expect("authorable");
        assert_eq!(
            ring, synthesised,
            "the authored ring must be the play area the compile would have made ({terrain})"
        );
        let verts: Vec<String> = ring
            .chunks_exact(2)
            .map(|c| format!("[{}, {}]", c[0], c[1]))
            .collect();
        let authored = format!(
            r#"[{{"id": "z1", "type": "{}", "label": "{WHOLE_TERRAIN_ZONE_LABEL}",
                      "shape": {{"polygon": [{}]}}}}]"#,
            whole_terrain_zone_type().expect("the schema declares `boundary`"),
            verts.join(", ")
        );
        let doc = compile_for(terrain, &terrain_payload(&authored));
        let boundaries: Vec<&str> = doc
            .zones
            .iter()
            .filter(|z| z.kind == "boundary")
            .map(|z| z.id.as_str())
            .collect();
        assert_eq!(
            boundaries,
            ["z1"],
            "the authored Play Area must BE the compiled play area — no second, synthesised \
                 z_bounds beside it ({terrain})"
        );
        assert_eq!(
            compiled_boundary_ring(&doc, "z1"),
            ring,
            "and its ring must survive the compile unchanged ({terrain})"
        );
        assert_eq!(
            doc.zones
                .iter()
                .find(|z| z.id == "z1")
                .map(|z| z.label.as_str()),
            Some(WHOLE_TERRAIN_ZONE_LABEL),
            "the author's label must reach the mod ({terrain})"
        );
    }
}

/// ═══ THE ANSWER REFUSES A WORLD IT WAS NOT BUILT FOR ═══
///
/// [`terrain_rect_ring`] is a spatial answer, so it carries its terrain and refuses bounds that
/// disagree with it. The failure this guards is not hypothetical: the editor reads the extent
/// through `terrain_bounds_of(core)` and the terrain key through the same doc, and any future
/// path that resolved one from a manifest, a cached value or a remembered `12800` would produce
/// a "whole-terrain" zone that is not the terrain — invisible in the editor, wrong in game.
#[test]
fn terrain_rect_ring_refuses_bounds_that_are_not_this_terrain() {
    use website_map_engine::data::scenario::compile::terrain_bounds;
    let everon = terrain_bounds("everon");
    let arland = terrain_bounds("arland");
    assert!(terrain_rect_ring("everon", everon).is_some());
    assert!(terrain_rect_ring("arland", arland).is_some());
    // The classic mix-up: the right terrain, the OTHER terrain's extent.
    assert_eq!(
        terrain_rect_ring("arland", everon),
        None,
        "arland must refuse Everon's 12800² rect"
    );
    assert_eq!(
        terrain_rect_ring("everon", arland),
        None,
        "everon must refuse Arland's 4096² rect"
    );
    // And a hand-built extent that is merely CLOSE is still not the map.
    assert_eq!(
        terrain_rect_ring("everon", [0.0, 0.0, 12_800.0, 12_799.9]),
        None,
        "an extent one grid step short of the terrain is not the terrain"
    );
}

/// ═══ THE RECT RULE, FIRED ═══
///
/// The [`radius_survives_compile`] twin: real bounds pass; a rect collapsed on either axis, or
/// thinner than the 0.1 m grid, or carrying a non-finite corner, is refused rather than
/// committed as a zone the save would reject. Stated on the bounds alone (not through
/// `terrain_rect_ring`, whose world guard would answer first and hide it), so the rule itself is
/// what is under test.
#[test]
fn terrain_rect_rule_fires() {
    use website_map_engine::data::scenario::compile::terrain_bounds;
    for terrain in SHIPPED_TERRAINS {
        assert!(
            terrain_rect_is_authorable(terrain_bounds(terrain)),
            "{terrain}'s real rect must be authorable"
        );
    }
    let good = terrain_bounds("everon");
    // Zero width, then zero height.
    assert!(!terrain_rect_is_authorable([
        good[0], good[1], good[0], good[3]
    ]));
    assert!(!terrain_rect_is_authorable([
        good[0], good[1], good[2], good[1]
    ]));
    // Sub-grid width: `round_coord(0.04) == 0.0`, the same collapse a degenerate circle faces.
    assert!(
        !terrain_rect_is_authorable([0.0, 0.0, ZONE_GRID_M / 2.5, 12_800.0]),
        "a rect thinner than the grid quantises two corners together"
    );
    // Inverted (max behind min) is not "a rect drawn the other way", it is no rect at all.
    assert!(!terrain_rect_is_authorable([
        0.0, 0.0, -12_800.0, -12_800.0
    ]));
    // A non-finite corner must never reach the document.
    assert!(!terrain_rect_is_authorable([0.0, 0.0, f64::NAN, 12_800.0]));
    assert!(!terrain_rect_is_authorable([
        0.0,
        0.0,
        f64::INFINITY,
        12_800.0
    ]));
    // The rule is a gate, not a latch: the untouched bound is authorable again.
    assert!(terrain_rect_is_authorable(good));
}

/// The type is read from the schema, like every other `zone.type` this panel offers — an
/// invented value saves 201 and then 500s `/compiled` (T-581).
#[test]
fn whole_terrain_zone_type_comes_from_the_schema() {
    let t = whole_terrain_zone_type().expect("the schema must declare `boundary`");
    assert_eq!(t, "boundary");
    assert!(
        zone_types().contains(&t),
        "the whole-terrain type must be one the schema declares"
    );
    assert_eq!(WHOLE_TERRAIN_ZONE_LABEL, "Play Area");
}

/// ═══ THE AFFORDANCE IS WIRED — BUTTON → COMMAND → DOCUMENT ═══
///
/// `operations::entity` is `#![cfg(target_arch = "wasm32")]`, so no native test can link the
/// command; this is the source pin `every_t211_mutator_has_a_caller` set the precedent for. It
/// runs on `class_r_scrub::live_code` (comments deleted, string literals BLANKED) so a needle
/// can only be satisfied by a real call — never by the comment that describes it, and never by a
/// mention inside a string. The button's user-visible copy is pinned separately on `live_source`,
/// which keeps literals, because copy that ships IS code.
///
/// Without this the affordance could rot to a button that calls nothing, which is the T-582
/// defect in miniature.
#[test]
fn whole_terrain_affordance_is_wired() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};
    // The command is this panel's own: the ring, the type and the label are the panel's
    // vocabulary, so the haystack is the panel's production half.
    let ops = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/zones_panel.rs"
    )));
    let body = only_body(&ops, "pub fn add_whole_terrain_zone() -> Option<String>");

    // It reads the LIVE terrain off the document — both halves, so the world guard has two
    // things to compare — and never a literal extent.
    assert!(
        body.contains("terrain_key_of(core)") && body.contains("terrain_bounds_of(core)"),
        "T-702: the command must read the live terrain KEY and its BOUNDS from the doc, so the \
             rect it authors is the rect the compile reads"
    );
    // The rect is built through the pure, world-guarded helper here — not open-coded.
    assert!(
        body.contains("terrain_rect_ring("),
        "T-702: the ring must come from terrain_rect_ring (which refuses bounds that are not \
             this terrain), not from four corners spelled in the command"
    );
    // The create is the ONE-transaction labelled mutator. `add_polygon_zone` +
    // `set_zone_label` would be two undo steps
    // (store.rs `a_labelled_polygon_zone_create_is_one_undo_step`), and the acceptance is one.
    assert!(
        body.contains("core.add_polygon_zone_labelled("),
        "T-702: one button = one undo step, so the create must be the single-txn labelled \
             mutator — never add_polygon_zone followed by set_zone_label"
    );
    assert!(
        !body.contains("core.set_zone_label("),
        "T-702: a second mutator here is a second undo step; the label rides the create"
    );
    // The label and the type are the shared, schema-checked values — not second literals.
    assert!(
        body.contains("WHOLE_TERRAIN_ZONE_LABEL") && body.contains("whole_terrain_zone_type()"),
        "T-702: label and type must be the shared constants, not re-spelled in the command"
    );

    // The panel button CALLS the command and selects what it returns, so Attributes open on the
    // zone the author just made.
    //
    // Sliced to the WASM half before scrubbing: this file ships two `zones_panel` definitions —
    // the real one and the `cfg(not(wasm32))` native stub that returns `().into_any()` — and
    // `live_code` keeps BOTH (it resolves dead `cfg` items, but a native-only item is not dead
    // on the native build that runs this test). `only_body` refuses that ambiguity by design
    // (T-601), and rightly: a pin that guessed between a real panel and an empty stub is a pin
    // that could be greened by the stub. The anchor is the stub's own doc line, taken on the RAW
    // source because scrubbing deletes comments.
    let raw_panel = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/zones_panel.rs"
    ));
    // Split so this test's own source does not contain the anchor it searches for — the file it
    // reads is the file it lives in, and a whole literal here would be a second match (measured:
    // it was, first try). Same idiom as `editor_live_from_page`.
    let stub_anchor = format!("{}{}", "/// Native shell: no ", "document, so no zones.");
    assert_eq!(
        raw_panel.matches(stub_anchor.as_str()).count(),
        1,
        "T-702: the wasm/native slice anchor must be unambiguous"
    );
    let wasm_half = &raw_panel[..raw_panel.find(stub_anchor.as_str()).expect("counted above")];
    let panel = live_code(wasm_half);
    let panel_copy = live_source(wasm_half);
    assert!(
        panel.contains("add_whole_terrain_zone()"),
        "T-702: the Zones panel must CALL the command (the T-582 no-caller defect must not recur)"
    );
    assert!(
        panel.contains("selected.set(Some(id))"),
        "T-702: the panel must select the new zone id, which is what opens its Attributes"
    );

    // ═══ "ATTRIBUTES OPEN" IS A CONSEQUENCE OF THE SELECTION, NOT A SECOND ACT ═══
    //
    // The panel has no "attributes are open" flag to set: the Attributes block renders exactly
    // when `selected` names a row `zone_rows()` still returns. So `selected.set(Some(id))` on
    // an id the command just minted IS the panel opening — there is no third condition that a
    // create could satisfy for a hand-drawn zone and miss for this one. Pinned structurally
    // because the DOM half is only observable in a browser (see the slice's manual checklist).
    //
    // Matched EXACTLY, not by `contains`. A `contains` pin passes straight through an added
    // condition — measured: it stayed green while the gate was perturbed to
    // `selected.get().filter(|_| attrs_open)` with `attrs_open = false`, i.e. a panel that never
    // opens at all. So the whole guard region is normalised and compared whole.
    let gate = only_body(
            &panel,
            "pub(crate) fn zones_panel(doc_tick: RwSignal<u64>, selected: RwSignal<Option<String>>) -> AnyView",
        );
    let opens_at = gate
        .find("zone_attributes(")
        .expect("T-702: the panel must render the Attributes block");
    let head = "let Some(id) = selected.get()";
    let from = gate[..opens_at]
        .rfind(head)
        .expect("T-702: Attributes must be gated on the selection");
    let squash = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    assert_eq!(
        squash(&gate[from..opens_at]),
        squash(
            "let Some(id) = selected.get() else { return ().into_any(); };
                 let Some(z) = engine_ops::zone_rows().into_iter().find(|r| r.id == id) else {
                     return ().into_any();
                 };"
        ),
        "T-702: Attributes must be gated on `selected` plus the live row and NOTHING else — any \
             third condition is a second thing the whole-terrain create would have to satisfy, and \
             selecting the new id would stop being enough to open the panel"
    );
    assert!(
        panel_copy.contains("Whole-terrain zone"),
        "T-702: the affordance must be a labelled control an author can find"
    );
}
