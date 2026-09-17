//! Zones panel zone geometry and schema tests.

use super::{
    circle_from_clicks, humanize_key, humanize_token, polygon_flat, polygon_is_committable,
    radius_survives_compile, round_coord, terrain_rect_is_authorable, terrain_rect_ring,
    whole_terrain_zone_type, zone_rule_fields, zone_types, ZoneRuleKind, MIN_AUTHORABLE_RADIUS_M,
    MISSION_SCHEMA, WHOLE_TERRAIN_ZONE_LABEL, ZONE_GRID_M,
};

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
    assert_eq!(round_coord(0.04), 0.0);
    assert_eq!(round_coord(0.05), 0.1);
}

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
    assert!(!radius_survives_compile(0.04));
    assert!(!radius_survives_compile(0.0));
    assert!(!radius_survives_compile(-5.0));
    assert!(!radius_survives_compile(f64::NAN));
    assert!(radius_survives_compile(250.0));
}

#[test]
fn circle_refuses_the_click_without_drag() {
    assert_eq!(circle_from_clicks(100.0, 200.0, 100.0, 200.0), None);
    assert_eq!(circle_from_clicks(0.0, 0.0, 0.04, 0.0), None);
    let (x, z, r) = circle_from_clicks(10.0, 20.0, 13.0, 24.0).expect("a real drag commits");
    assert!((x - 10.0).abs() < f64::EPSILON && (z - 20.0).abs() < f64::EPSILON);
    assert!((r - 5.0).abs() < 1e-12, "3-4-5 triangle: r = 5, got {r}");
    assert_eq!(circle_from_clicks(f64::NAN, 0.0, 1.0, 1.0), None);
}

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
    match kind("targetAlias") {
        ZoneRuleKind::Text { pattern, .. } => assert_eq!(
            pattern.as_deref(),
            Some("^(kit|comp|veh|preset|layer|prop|item):[a-z0-9_]+$"),
            "the $ref into $defs/alias must be resolved"
        ),
        other => panic!("targetAlias must resolve to Text, got {other:?}"),
    }
    assert!(
        fields.iter().all(|f| !f.doc.is_empty()),
        "each control shows the schema's own description"
    );
}

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

#[test]
fn every_t211_mutator_has_a_caller() {
    let ops = [
        crate::v2::core::test_support::editor_operations::ENTITY,
        crate::v2::core::test_support::editor_operations::DOMAIN_ENTITY,
    ]
    .concat();
    #[allow(non_snake_case)]
    let OPS: &str = &ops;
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
    assert!(
        OPS.contains("begin_zone_reshape"),
        "reshape must be reachable, not just create"
    );
}

#[test]
fn labels_never_replace_tokens() {
    assert_eq!(
        humanize_token("objective_hold_until"),
        "Objective hold until"
    );
    assert_eq!(humanize_token("spawn"), "Spawn");
    assert_eq!(humanize_key("warnEverySeconds"), "Warn every seconds");
    assert_eq!(humanize_key("penalty"), "Penalty");
    for f in zone_rule_fields() {
        assert_ne!(
            humanize_key(&f.key),
            f.key,
            "label must differ from wire key"
        );
    }
}

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

#[test]
fn t792_escape_arm_cancels_the_zone_draw() {
    let ed = editor_live_from_page();
    assert!(
            ed.contains("armed_placement::cancel_zone_draw()"),
            "T-792: the editor keydown must call armed_placement::cancel_zone_draw() (the ONE cancel a \
             multi-click draw honours) — cancel_pending alone leaves the draft armed"
        );
    assert!(
        ed.contains("code().as_str()")
            && ed.contains("armed_placement::has_pending()")
            && ed.contains("armed_placement::cancel_zone_draw()"),
        "T-792: the zone-draw cancel must live in the ONE shared keydown Escape arm, beside the \
             armed-place (has_pending) cancel — no second window keydown listener"
    );
}

#[test]
fn t792_zone_cancel_consumes_the_press() {
    let ed = editor_live_from_page();
    assert!(
        ed.contains("let zone_draw_acted =") && ed.contains("|| zone_draw_acted"),
        "T-792: the draw-cancel result must join the Escape arm's handled `||` chain, so a real \
             cancel consumes the press (one Esc, one layer)"
    );
}

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
    assert!(
        body.contains("Some(Pending::Zone(_))"),
        "T-792: cancel_zone_draw must clear on Pending::Zone(_) regardless of collection, so a \
             trigger draw (the second consumer) is cancelled by the same call"
    );
}

const SHIPPED_TERRAINS: [&str; 2] = ["everon", "arland"];

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
    assert_eq!(
        doc.schema_version, "1.1",
        "the mod-document format this golden was written against"
    );
    doc
}

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
    assert_ne!(
        terrain_rect_ring("everon", terrain_bounds("everon")),
        terrain_rect_ring("arland", terrain_bounds("arland")),
        "everon 12800² and arland 4096² must not author the same ring"
    );
    for unknown in ["custom", "definitely-not-a-terrain", ""] {
        assert_eq!(
            terrain_rect_ring(unknown, terrain_bounds(unknown)),
            terrain_rect_ring("everon", terrain_bounds("everon")),
            "an unknown terrain must author Everon's rect, the fallback terrain_bounds takes"
        );
    }
}

#[test]
fn the_authored_play_area_becomes_the_compiled_play_area() {
    use website_map_engine::data::scenario::compile::terrain_bounds;
    for terrain in SHIPPED_TERRAINS {
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

#[test]
fn terrain_rect_ring_refuses_bounds_that_are_not_this_terrain() {
    use website_map_engine::data::scenario::compile::terrain_bounds;
    let everon = terrain_bounds("everon");
    let arland = terrain_bounds("arland");
    assert!(terrain_rect_ring("everon", everon).is_some());
    assert!(terrain_rect_ring("arland", arland).is_some());
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
    assert_eq!(
        terrain_rect_ring("everon", [0.0, 0.0, 12_800.0, 12_799.9]),
        None,
        "an extent one grid step short of the terrain is not the terrain"
    );
}

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
    assert!(!terrain_rect_is_authorable([
        good[0], good[1], good[0], good[3]
    ]));
    assert!(!terrain_rect_is_authorable([
        good[0], good[1], good[2], good[1]
    ]));
    assert!(
        !terrain_rect_is_authorable([0.0, 0.0, ZONE_GRID_M / 2.5, 12_800.0]),
        "a rect thinner than the grid quantises two corners together"
    );
    assert!(!terrain_rect_is_authorable([
        0.0, 0.0, -12_800.0, -12_800.0
    ]));
    assert!(!terrain_rect_is_authorable([0.0, 0.0, f64::NAN, 12_800.0]));
    assert!(!terrain_rect_is_authorable([
        0.0,
        0.0,
        f64::INFINITY,
        12_800.0
    ]));
    assert!(terrain_rect_is_authorable(good));
}

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

#[test]
fn whole_terrain_affordance_is_wired() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};
    let ops = live_code(super::ZONES_PANEL_SOURCE);
    let body = only_body(&ops, "pub fn add_whole_terrain_zone() -> Option<String>");

    assert!(
        body.contains("terrain_key_of(core)") && body.contains("terrain_bounds_of(core)"),
        "T-702: the command must read the live terrain KEY and its BOUNDS from the doc, so the \
             rect it authors is the rect the compile reads"
    );
    assert!(
        body.contains("terrain_rect_ring("),
        "T-702: the ring must come from terrain_rect_ring (which refuses bounds that are not \
             this terrain), not from four corners spelled in the command"
    );
    assert!(
        body.contains("core.add_polygon_zone_labelled("),
        "T-702: one button = one undo step, so the create must be the single-txn labelled \
             mutator — never add_polygon_zone followed by set_zone_label"
    );
    assert!(
        !body.contains("core.set_zone_label("),
        "T-702: a second mutator here is a second undo step; the label rides the create"
    );
    assert!(
        body.contains("WHOLE_TERRAIN_ZONE_LABEL") && body.contains("whole_terrain_zone_type()"),
        "T-702: label and type must be the shared constants, not re-spelled in the command"
    );

    let raw_panel = super::ZONES_PANEL_SOURCE;
    let stub_anchor = "#[cfg(not(target_arch = \"wasm32\"))]\npub(crate) fn zones_panel(";
    assert_eq!(
        raw_panel.matches(stub_anchor).count(),
        1,
        "T-702: the wasm/native slice anchor must be unambiguous"
    );
    let wasm_half = &raw_panel[..raw_panel.find(stub_anchor).expect("counted above")];
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
