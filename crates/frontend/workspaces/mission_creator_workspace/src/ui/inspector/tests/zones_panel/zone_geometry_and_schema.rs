//! Zones panel zone geometry and schema tests.

use super::{
    MIN_AUTHORABLE_RADIUS_M, MISSION_SCHEMA, WHOLE_TERRAIN_ZONE_LABEL, ZONE_GRID_M, ZoneRuleKind,
    circle_from_clicks, polygon_flat, polygon_is_committable, radius_survives_compile, round_coord,
    terrain_rect_is_authorable, terrain_rect_ring, whole_terrain_zone_type, zone_rule_fields,
    zone_types,
};

#[test]
fn zone_quantisation_rounds_to_a_tenth() {
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

fn compile_for(terrain: &str, payload: &[u8]) -> mission_compiler::ModMissionDocument {
    use mission_compiler::MissionMeta;
    use mission_compiler::flatten_to_mod_document;
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

fn compiled_boundary_ring(doc: &mission_compiler::ModMissionDocument, id: &str) -> Vec<f64> {
    use mission_model::compiled::mission::ModZoneShape;
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
    use mission_payload::terrain_bounds;
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
    use mission_payload::terrain_bounds;
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
    use mission_payload::terrain_bounds;
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
    use mission_payload::terrain_bounds;
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
