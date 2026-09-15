//! Role: Domain regression cases.
//! Position: `mission/validation/validator/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn v1_player_spawn_fires_when_a_faction_has_no_slots() {
    let findings = validate_editor_payload(&rule_v1_player_spawn().trip_fixture());
    let f = finding_for(&findings, "V1-PLAYER-SPAWN");
    assert_eq!(f.severity, Severity::Error);
    assert_eq!(f.primitive, Primitive::RequiredEntity);
    assert_eq!(f.subject, "/editor/slots");
    assert!(f.message.contains("nowhere for a player to spawn"), "{f:?}");
}

#[test]
fn v1_is_conditional_a_factionless_draft_does_not_fire() {
    for empty in [json!({}), json!({"editor": {"slots": []}})] {
        let rule = rule_v1_player_spawn();
        assert!(
            !rule.applies(&empty, &EvalContext::default()),
            "must not apply: {empty}"
        );
        assert!(
            validate_editor_payload(&empty)
                .iter()
                .all(|f| f.rule_id != "V1-PLAYER-SPAWN"),
            "V1 must stay silent on a factionless draft: {empty}"
        );
    }
}

#[test]
fn v1_does_not_fire_when_the_declared_faction_has_a_slot() {
    let p = json!({"editor": {
        "factions": [{"key": "BLUFOR", "name": "US", "squadIds": ["sq1"]}],
        "squads": [{"id": "sq1", "callsign": "Alpha", "slotIds": ["s1"]}],
        "slots": [{"id": "s1", "role": "RFL", "position": {"x": 100.0, "y": 100.0}}]
    }});
    assert!(
        validate_editor_payload(&p)
            .iter()
            .all(|f| f.rule_id != "V1-PLAYER-SPAWN"),
        "{p}"
    );
}

#[test]
fn v2_faction_max_fires_on_five_factions() {
    let findings = validate_editor_payload(&rule_v2_faction_max().trip_fixture());
    let f = finding_for(&findings, "V2-FACTION-MAX");
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.primitive, Primitive::Cardinality);
    assert_eq!(f.subject, "/editor/factions");
    assert!(f.message.contains("5 factions declared"), "{f:?}");
    assert!(f.message.contains("at most 4"), "{f:?}");
}

#[test]
fn v2_does_not_fire_at_exactly_four_factions() {
    let factions: Vec<Value> = ["BLUFOR", "OPFOR", "INDFOR", "CIV"]
        .iter()
        .map(|k| json!({"key": k, "name": k}))
        .collect();
    let p = json!({ "editor": { "factions": factions } });
    assert!(
        validate_editor_payload(&p)
            .iter()
            .all(|f| f.rule_id != "V2-FACTION-MAX"),
        "four is the ceiling, not over it: {p}"
    );
}

#[test]
fn v3_slot_in_bounds_fires_on_an_out_of_bounds_slot() {
    let findings = validate_editor_payload(&rule_v3_slot_in_bounds().trip_fixture());
    let f = finding_for(&findings, "V3-SLOT-IN-BOUNDS");
    assert_eq!(f.severity, Severity::Error);
    assert_eq!(f.primitive, Primitive::PerObjectInvariant);
    assert_eq!(f.subject, "/editor/slots/0/position");
    assert!(
        f.message.contains("outside the everon terrain bounds"),
        "{f:?}"
    );
    assert!(f.message.contains("20000.0"), "{f:?}");
}

#[test]
fn v3_returns_one_finding_per_offending_slot_never_early_exits() {
    let p = json!({"map": {"terrain": "everon"}, "editor": {"slots": [
        {"id": "a", "position": {"x": -5.0, "y": 100.0}},
        {"id": "b", "position": {"x": 100.0, "y": 100.0}},
        {"id": "c", "position": {"x": 100.0, "y": 99999.0}}
    ]}});
    let v3: Vec<Finding> = validate_editor_payload(&p)
        .into_iter()
        .filter(|f| f.rule_id == "V3-SLOT-IN-BOUNDS")
        .collect();
    assert_eq!(v3.len(), 2, "{v3:?}");
    assert_eq!(v3[0].subject, "/editor/slots/0/position");
    assert_eq!(v3[1].subject, "/editor/slots/2/position");
}

#[test]
fn v3_uses_the_authored_terrain_bounds_not_a_fixed_size() {
    let slot = json!({"id": "s", "position": {"x": 5000.0, "y": 5000.0}});
    let everon = json!({"map": {"terrain": "everon"}, "editor": {"slots": [slot.clone()]}});
    let arland = json!({"map": {"terrain": "arland"}, "editor": {"slots": [slot]}});
    assert!(
        validate_editor_payload(&everon)
            .iter()
            .all(|f| f.rule_id != "V3-SLOT-IN-BOUNDS"),
        "in-bounds on everon"
    );
    let arland_findings = validate_editor_payload(&arland);
    let f = finding_for(&arland_findings, "V3-SLOT-IN-BOUNDS");
    assert!(f.message.contains("arland"), "{f:?}");
}

#[test]
fn v3_ignores_slots_without_a_numeric_position() {
    let p = json!({"map": {"terrain": "everon"}, "editor": {"slots": [
        {"id": "a"},
        {"id": "b", "position": {"x": "nope", "y": 100.0}},
        {"id": "c", "position": {}}
    ]}});
    assert!(
        validate_editor_payload(&p)
            .iter()
            .all(|f| f.rule_id != "V3-SLOT-IN-BOUNDS"),
        "{p}"
    );
}

#[test]
fn v4_schema_version_fires_on_a_string_version() {
    let findings = validate_editor_payload(&rule_v4_schema_version().trip_fixture());
    let f = finding_for(&findings, "V4-SCHEMA-VERSION");
    assert_eq!(f.severity, Severity::Error);
    assert_eq!(f.primitive, Primitive::FieldShape);
    assert_eq!(f.subject, "/schemaVersion");
    assert!(f.message.contains("positive integer"), "{f:?}");
}

#[test]
fn v4_fires_on_zero_and_negative_and_fractional() {
    for bad in [json!(0), json!(-1), json!(1.5)] {
        let p = json!({ "schemaVersion": bad });
        assert!(
            validate_editor_payload(&p)
                .iter()
                .any(|f| f.rule_id == "V4-SCHEMA-VERSION"),
            "must fire on schemaVersion={bad}"
        );
    }
}

#[test]
fn v4_accepts_absent_and_a_valid_positive_integer() {
    for ok in [
        json!({}),
        json!({"schemaVersion": 1}),
        json!({"schemaVersion": 2}),
    ] {
        assert!(
            validate_editor_payload(&ok)
                .iter()
                .all(|f| f.rule_id != "V4-SCHEMA-VERSION"),
            "must accept {ok}"
        );
    }
}

#[test]
fn a_clean_realistic_payload_produces_no_findings() {
    let p = clean_orbat_payload();
    assert!(
        validate_editor_payload(&p).is_empty(),
        "{:?}",
        validate_editor_payload(&p)
    );
}

#[test]
fn every_seed_rule_has_a_distinct_id_and_a_known_primitive() {
    let reg = default_registry();
    let mut ids: Vec<&str> = reg.rules().iter().map(Rule::id).collect();
    let count = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), count, "rule ids must be unique");

    let prims: Vec<Primitive> = reg.rules().iter().map(Rule::primitive).collect();
    for want in [
        Primitive::RequiredEntity,
        Primitive::Cardinality,
        Primitive::PerObjectInvariant,
        Primitive::FieldShape,
    ] {
        assert!(prims.contains(&want), "seed missing a {} rule", want.tag());
    }
}

#[test]
fn engine_self_check_passes_for_the_seed_registry() {
    default_registry()
        .self_check()
        .expect("every seed rule must fire on its trip fixture");
    default_registry().assert_self_check();
}

#[test]
fn engine_self_check_catches_a_rule_that_cannot_fire() {
    fn dead_eval(_r: &Rule, _p: &Value, _c: &EvalContext) -> Vec<Finding> {
        Vec::new()
    }
    let dead = Rule {
        id: "DEAD-RULE",
        severity: Severity::Error,
        primitive: Primitive::FieldShape,
        applies: |_, _| true,
        eval: dead_eval,
        trip_fixture: || json!({"anything": true}),
        trip_context: no_trip_context,
    };
    let reg = Registry::new(vec![dead]);
    let err = reg
        .self_check()
        .expect_err("a rule that never fires must fail self-check");
    assert_eq!(err.len(), 1);
    assert_eq!(err[0].rule_id, "DEAD-RULE");
    assert!(err[0].reason.contains("gone silent"), "{:?}", err[0]);
}

#[test]
fn engine_self_check_catches_a_rule_whose_gate_excludes_its_own_trip() {
    let misgated = Rule {
        id: "MISGATED-RULE",
        severity: Severity::Warning,
        primitive: Primitive::RequiredEntity,
        applies: |_, _| false,
        eval: |rule, _, _| vec![rule.finding("unreachable".into(), "/x".into())],
        trip_fixture: || json!({}),
        trip_context: no_trip_context,
    };
    let err = Registry::new(vec![misgated])
        .self_check()
        .expect_err("a rule gated off from its own trip must fail self-check");
    assert_eq!(err[0].rule_id, "MISGATED-RULE");
    assert!(err[0].reason.contains("`applies` gate"), "{:?}", err[0]);
}

#[test]
#[should_panic(expected = "duplicate rule id")]
fn registry_rejects_duplicate_ids() {
    let _ = Registry::new(vec![rule_v4_schema_version(), rule_v4_schema_version()]);
}

#[test]
#[should_panic(expected = "self-check failed")]
fn assert_self_check_panics_loudly_on_a_dead_rule() {
    let dead = Rule {
        id: "DEAD",
        severity: Severity::Error,
        primitive: Primitive::FieldShape,
        applies: |_, _| true,
        eval: |_, _, _| Vec::new(),
        trip_fixture: || json!({}),
        trip_context: no_trip_context,
    };
    Registry::new(vec![dead]).assert_self_check();
}

#[test]
fn evaluate_returns_all_findings_across_rules_never_early_exits() {
    let factions: Vec<Value> = (0..5)
        .map(|i| json!({"key": format!("S{i}"), "name": format!("S{i}")}))
        .collect();
    let p = json!({
        "schemaVersion": "bad",
        "map": {"terrain": "everon"},
        "editor": {
            "factions": factions,
            "slots": [{"id": "s", "position": {"x": 99999.0, "y": 1.0}}]
        }
    });
    let findings = validate_editor_payload(&p);
    for want in ["V2-FACTION-MAX", "V3-SLOT-IN-BOUNDS", "V4-SCHEMA-VERSION"] {
        assert!(
            findings.iter().any(|f| f.rule_id == want),
            "missing {want} in {findings:?}"
        );
    }
}

#[test]
fn orbat_slot_resolves_fires_when_a_slot_resolves_neither() {
    let findings = validate_editor_payload(&rule_orbat_slot_resolves().trip_fixture());
    let f = finding_for(&findings, "ORBAT-SLOT-RESOLVES");
    assert_eq!(f.severity, Severity::Error);
    assert_eq!(f.primitive, Primitive::PerObjectInvariant);
    assert_eq!(f.subject, "/editor/slots/0");
    assert_eq!(f.subject_id.as_deref(), Some("s1"));
    assert!(f.message.contains("resolves neither"), "{f:?}");
}

#[test]
fn orbat_slot_resolves_distinguishes_missing_role_from_missing_squad() {
    let p = json!({"editor": {
        "squads": [{"id": "sq1", "callsign": "A", "name": "A", "slotIds": ["s2"], "leaderSlotId": "s2"}],
        "slots": [
            {"id": "s1", "role": "RFL"},
            {"id": "s2", "role": ""}
        ]
    }});
    let findings = validate_editor_payload(&p);
    let fs: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.rule_id == "ORBAT-SLOT-RESOLVES")
        .collect();

    let s1 = fs
        .iter()
        .find(|f| f.subject_id.as_deref() == Some("s1"))
        .expect("a finding for s1");
    let s2 = fs
        .iter()
        .find(|f| f.subject_id.as_deref() == Some("s2"))
        .expect("a finding for s2");
    assert!(s1.message.contains("not filed under any squad"), "{s1:?}");
    assert!(s2.message.contains("has no role"), "{s2:?}");
}

#[test]
fn orbat_slot_resolves_is_conditional_on_a_declared_orbat() {
    for p in [
        json!({}),
        json!({"editor": {"slots": [{"id": "s1", "role": ""}]}}),
    ] {
        assert!(
            !rule_orbat_slot_resolves().applies(&p, &EvalContext::default()),
            "must not apply: {p}"
        );
        assert!(
            validate_editor_payload(&p)
                .iter()
                .all(|f| f.rule_id != "ORBAT-SLOT-RESOLVES"),
            "must stay silent without an ORBAT: {p}"
        );
    }
}

#[test]
fn orbat_identity_filled_fires_on_a_squad_with_no_callsign_and_no_name() {
    let findings = validate_editor_payload(&rule_orbat_identity_filled().trip_fixture());
    let f = finding_for(&findings, "ORBAT-IDENTITY-FILLED");
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.primitive, Primitive::PerObjectInvariant);
    assert_eq!(f.subject, "/editor/squads/0");
    assert_eq!(f.subject_id.as_deref(), Some("sq1"));
    assert!(f.message.contains("no callsign and no name"), "{f:?}");
}

#[test]
fn orbat_identity_filled_accepts_either_a_callsign_or_a_name() {
    for sq in [
        json!({"id": "sq1", "callsign": "Alpha", "name": "", "slotIds": []}),
        json!({"id": "sq1", "callsign": "  ", "name": "Alpha 1-1", "slotIds": []}),
    ] {
        let p = json!({"editor": {"squads": [sq.clone()]}});
        assert!(
            validate_editor_payload(&p)
                .iter()
                .all(|f| f.rule_id != "ORBAT-IDENTITY-FILLED"),
            "a partial identity must pass: {sq}"
        );
    }
}

#[test]
fn orbat_squad_has_leader_fires_on_a_manned_squad_with_no_leader() {
    let findings = validate_editor_payload(&rule_orbat_squad_has_leader().trip_fixture());
    let f = finding_for(&findings, "ORBAT-SQUAD-HAS-LEADER");
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.primitive, Primitive::PerObjectInvariant);
    assert_eq!(f.subject, "/editor/squads/0");
    assert_eq!(f.subject_id.as_deref(), Some("sq1"));
    assert!(f.message.contains("no leader"), "{f:?}");
}

#[test]
fn orbat_squad_has_leader_ignores_an_empty_squad_and_accepts_a_valid_leader() {
    let ok = json!({"editor": {
        "squads": [
            {"id": "sq-empty", "callsign": "E", "name": "E", "slotIds": []},
            {"id": "sq-led", "callsign": "L", "name": "L", "slotIds": ["s1"], "leaderSlotId": "s1"}
        ],
        "slots": [{"id": "s1", "role": "SL"}]
    }});
    assert!(
        validate_editor_payload(&ok)
            .iter()
            .all(|f| f.rule_id != "ORBAT-SQUAD-HAS-LEADER"),
        "empty + validly-led squads must pass: {:?}",
        validate_editor_payload(&ok)
    );
    let foreign = json!({"editor": {
        "squads": [{"id": "sq1", "callsign": "A", "name": "A", "slotIds": ["s1"], "leaderSlotId": "s2"}],
        "slots": [{"id": "s1", "role": "SL"}]
    }});
    assert!(
        validate_editor_payload(&foreign)
            .iter()
            .any(|f| f.rule_id == "ORBAT-SQUAD-HAS-LEADER"),
        "a leaderSlotId outside the squad is leaderless"
    );
}

#[test]
fn orbat_callsign_unique_fires_on_two_squads_sharing_a_callsign_on_one_side() {
    let findings = validate_editor_payload(&rule_orbat_callsign_unique().trip_fixture());
    let f = finding_for(&findings, "ORBAT-CALLSIGN-UNIQUE");
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.primitive, Primitive::PerObjectInvariant);

    assert_eq!(f.subject, "/editor/squads/1");
    assert_eq!(f.subject_id.as_deref(), Some("sq2"));
    assert!(f.message.contains("unique within a side"), "{f:?}");
}

#[test]
fn orbat_callsign_unique_does_not_fire_across_different_sides() {
    let p = json!({"editor": {
        "factions": [
            {"key": "BLUFOR", "name": "US", "squadIds": ["sq1"]},
            {"key": "OPFOR", "name": "SOV", "squadIds": ["sq2"]}
        ],
        "squads": [
            {"id": "sq1", "callsign": "Alpha", "name": "A", "slotIds": []},
            {"id": "sq2", "callsign": "Alpha", "name": "B", "slotIds": []}
        ]
    }});
    assert!(
        validate_editor_payload(&p)
            .iter()
            .all(|f| f.rule_id != "ORBAT-CALLSIGN-UNIQUE"),
        "same callsign on two DIFFERENT sides must not fire: {:?}",
        validate_editor_payload(&p)
    );
}

#[test]
fn orbat_template_coverage_fires_when_a_required_role_is_unfilled() {
    let findings = validate_editor_payload(&rule_orbat_template_coverage().trip_fixture());
    let f = finding_for(&findings, "ORBAT-TEMPLATE-COVERAGE");
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.primitive, Primitive::PerObjectInvariant);
    assert_eq!(f.subject, "/editor/squads/0");
    assert_eq!(f.subject_id.as_deref(), Some("sq1"));
    assert!(f.message.contains("MED"), "{f:?}");
    assert!(f.message.contains("template"), "{f:?}");
}

#[test]
fn orbat_template_coverage_skips_a_squad_with_no_template() {
    let p = json!({"editor": {
        "squads": [{"id": "sq1", "callsign": "A", "name": "A", "slotIds": ["s1"], "leaderSlotId": "s1"}],
        "slots": [{"id": "s1", "role": "RFL"}]
    }});
    assert!(
        validate_editor_payload(&p)
            .iter()
            .all(|f| f.rule_id != "ORBAT-TEMPLATE-COVERAGE"),
        "a squad with no template must skip coverage: {:?}",
        validate_editor_payload(&p)
    );
}

#[test]
fn orbat_template_coverage_passes_when_every_required_role_is_filled() {
    let p = json!({"editor": {
        "squads": [{
            "id": "sq1", "callsign": "A", "name": "A",
            "slotIds": ["s1", "s2"], "leaderSlotId": "s1",
            "template": {"requiredRoles": ["SL", "MED"]}
        }],
        "slots": [{"id": "s1", "role": "SL"}, {"id": "s2", "role": "MED"}]
    }});
    assert!(
        validate_editor_payload(&p)
            .iter()
            .all(|f| f.rule_id != "ORBAT-TEMPLATE-COVERAGE"),
        "full coverage must pass: {:?}",
        validate_editor_payload(&p)
    );
}

#[test]
fn an_empty_orbat_produces_no_orbat_findings_and_does_not_panic() {
    for p in [
        json!({}),
        json!({"editor": {}}),
        json!({"editor": {"factions": [], "squads": [], "slots": []}}),
    ] {
        let findings = validate_editor_payload(&p);
        for id in [
            "ORBAT-SLOT-RESOLVES",
            "ORBAT-IDENTITY-FILLED",
            "ORBAT-SQUAD-HAS-LEADER",
            "ORBAT-CALLSIGN-UNIQUE",
            "ORBAT-TEMPLATE-COVERAGE",
        ] {
            assert!(
                findings.iter().all(|f| f.rule_id != id),
                "{id} must be silent on {p}"
            );
        }
    }
}

#[test]
fn orbat_rules_never_panic_on_garbage() {
    let garbage = [
        json!(null),
        json!(42),
        json!("a string, not an object"),
        json!([]),
        json!({"editor": 7}),
        json!({"editor": {"squads": "nope", "slots": 3, "factions": {}}}),
        json!({"editor": {"squads": [null, 5, "x", {}]}}),
        json!({"editor": {"slots": [null, 9, {"id": 5, "role": []}]}}),
        json!({"editor": {
            "factions": [{"key": null, "squadIds": [1, 2, {}, "sq1"]}],
            "squads": [{"id": null, "callsign": 5, "name": [], "slotIds": "x", "leaderSlotId": {}}],
            "slots": [{"id": 1, "role": 2}]
        }}),
        json!({"editor": {"squads": [{
                "id": "sq1", "slotIds": [null, 3, "s1"],
                "template": {"requiredRoles": [null, 7, "SL", ""]}
            }], "slots": [{"id": "s1", "role": null}]}}),
        json!({"schemaVersion": {"nested": [1, 2, 3]}, "map": {"terrain": []}}),
    ];
    let reg = default_registry();
    for p in garbage {
        let _ = reg.evaluate(&p);
        let _ = validate_editor_payload(&p);
    }
}

#[test]
fn t657_rules_are_registered_and_self_check_passes() {
    let reg = default_registry();
    let ids: Vec<&str> = reg.rules().iter().map(Rule::id).collect();
    for want in [
        "ORBAT-SLOT-RESOLVES",
        "ORBAT-IDENTITY-FILLED",
        "ORBAT-SQUAD-HAS-LEADER",
        "ORBAT-CALLSIGN-UNIQUE",
        "ORBAT-TEMPLATE-COVERAGE",
    ] {
        assert!(ids.contains(&want), "registry missing {want}");
    }

    reg.self_check()
        .expect("every rule (seed + T-657) must fire on its trip fixture");
}

#[test]
fn perturb_and_restore_fires_exactly_the_leader_rule() {
    let clean = clean_orbat_payload();
    assert!(
        validate_editor_payload(&clean).is_empty(),
        "baseline must be green: {:?}",
        validate_editor_payload(&clean)
    );

    let mut broken = clean.clone();
    broken["editor"]["squads"][0]
        .as_object_mut()
        .unwrap()
        .remove("leaderSlotId");
    let findings = validate_editor_payload(&broken);
    let leader: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.rule_id == "ORBAT-SQUAD-HAS-LEADER")
        .collect();
    assert_eq!(
        leader.len(),
        1,
        "exactly one leaderless finding: {findings:?}"
    );
    assert_eq!(leader[0].subject_id.as_deref(), Some("sq1"));

    for other in [
        "ORBAT-SLOT-RESOLVES",
        "ORBAT-IDENTITY-FILLED",
        "ORBAT-CALLSIGN-UNIQUE",
        "ORBAT-TEMPLATE-COVERAGE",
    ] {
        assert!(
            findings.iter().all(|f| f.rule_id != other),
            "only the leader rule should fire; {other} also fired: {findings:?}"
        );
    }

    assert!(
        validate_editor_payload(&clean).is_empty(),
        "restoring must return to green"
    );
}

#[test]
fn asset_resolves_fires_on_an_unknown_placed_asset() {
    let rule = rule_asset_resolves();
    let ctx = rule
        .trip_context()
        .expect("ASSET-RESOLVES declares a trip_context");
    let findings = default_registry().evaluate_with_context(&rule.trip_fixture(), &ctx);
    let f = finding_for(&findings, "ASSET-RESOLVES");
    assert_eq!(f.severity, Severity::Error);
    assert_eq!(f.primitive, Primitive::PerObjectInvariant);
    assert_eq!(f.subject, "/editor/slots/0/assetId");
    assert_eq!(f.subject_id.as_deref(), Some("s1"));
    assert!(
        f.message.contains("does not resolve in the live catalogue"),
        "{f:?}"
    );
    assert!(
        f.message.contains("{ABC}Prefabs/Characters/Ghost.et"),
        "{f:?}"
    );
}

#[test]
fn asset_resolves_passes_when_every_placed_asset_is_in_the_catalogue() {
    let p = json!({"editor": {"slots": [
        {"id": "s1", "role": "RFL", "assetId": "{ABC}Prefabs/Characters/Ghost.et"}
    ]}});
    let ctx = ctx_with(&["{ABC}Prefabs/Characters/Ghost.et"]);
    assert!(
        default_registry()
            .evaluate_with_context(&p, &ctx)
            .iter()
            .all(|f| f.rule_id != "ASSET-RESOLVES"),
        "a resolvable asset must not fire: {:?}",
        default_registry().evaluate_with_context(&p, &ctx)
    );
}

#[test]
fn asset_resolves_skips_when_no_catalogue_is_supplied() {
    let p = json!({"editor": {"slots": [
        {"id": "s1", "role": "RFL", "assetId": "{ABC}Prefabs/Characters/Ghost.et"}
    ]}});
    let rule = rule_asset_resolves();

    assert!(
        !rule.applies(&p, &EvalContext::default()),
        "must not apply without a catalogue"
    );

    assert!(
        validate_editor_payload(&p)
            .iter()
            .all(|f| f.rule_id != "ASSET-RESOLVES"),
        "evaluate() (default ctx) must not fire ASSET-RESOLVES"
    );
    assert!(
        default_registry()
            .evaluate_with_context(&p, &EvalContext::default())
            .iter()
            .all(|f| f.rule_id != "ASSET-RESOLVES"),
        "explicit empty context must not fire ASSET-RESOLVES"
    );
}

#[test]
fn asset_resolves_applies_but_flags_all_when_the_catalogue_is_empty_but_present() {
    let p = json!({"editor": {"slots": [
        {"id": "s1", "role": "RFL", "assetId": "{ABC}X.et"}
    ]}});
    let ctx = EvalContext::default().with_known_asset_ids(HashSet::new());
    assert!(
        rule_asset_resolves().applies(&p, &ctx),
        "Some(empty) applies"
    );
    let findings = default_registry().evaluate_with_context(&p, &ctx);
    assert!(
        findings.iter().any(|f| f.rule_id == "ASSET-RESOLVES"),
        "an empty-but-present catalogue resolves nothing: {findings:?}"
    );
}

#[test]
fn asset_resolves_resolves_alias_forms_for_vehicles_and_entities() {
    let p = json!({
        "editor": {"slots": [
            {"id": "s1", "role": "RFL", "assetId": "{ABC}Prefabs/Characters/US_Rifleman.et"}
        ]},
        "vehicles": [
            {"id": "v1", "resourceName": "{ABC}Prefabs/Vehicles/Humvee.et"}
        ],
        "entities": [
            {"id": "e1", "alias": "prop:ammo_crate", "resourceName": "{ABC}Prefabs/Props/AmmoBox.et"}
        ]
    });
    let ctx = ctx_with(&[
        "{ABC}Prefabs/Characters/US_Rifleman.et",
        "{ABC}Prefabs/Vehicles/Humvee.et",
        "prop:ammo_crate",
    ]);
    let findings = default_registry().evaluate_with_context(&p, &ctx);
    assert!(
        findings.iter().all(|f| f.rule_id != "ASSET-RESOLVES"),
        "all three forms must resolve: {findings:?}"
    );
}

#[test]
fn asset_resolves_fires_per_unresolved_reference_across_kinds_never_early_exits() {
    let p = json!({
        "editor": {"slots": [
            {"id": "s1", "role": "RFL", "assetId": "{ABC}Known.et"}
        ]},
        "vehicles": [
            {"id": "v1", "resourceName": "{ABC}GoneVehicle.et"}
        ],
        "entities": [
            {"id": "e1", "alias": "comp:gone_comp", "resourceName": "{ABC}GoneObj.et"}
        ]
    });
    let ctx = ctx_with(&["{ABC}Known.et"]);
    let all = default_registry().evaluate_with_context(&p, &ctx);
    let asset_findings: Vec<&Finding> = all
        .iter()
        .filter(|f| f.rule_id == "ASSET-RESOLVES")
        .collect();
    assert_eq!(asset_findings.len(), 2, "{asset_findings:?}");
    let v = asset_findings
        .iter()
        .find(|f| f.subject_id.as_deref() == Some("v1"))
        .expect("a finding for the vehicle v1");
    let e = asset_findings
        .iter()
        .find(|f| f.subject_id.as_deref() == Some("e1"))
        .expect("a finding for the entity e1");
    assert_eq!(v.subject, "/vehicles/0/resourceName");
    assert!(
        v.message.contains("prefab"),
        "vehicle id is a raw prefab: {v:?}"
    );
    assert_eq!(e.subject, "/entities/0/alias");
    assert!(e.message.contains("alias"), "object id is an alias: {e:?}");
}

#[test]
fn asset_resolves_ignores_placements_with_no_asset_id() {
    let p = json!({
        "editor": {"slots": [
            {"id": "s1", "role": "RFL"},
            {"id": "s2", "role": "MED", "assetId": ""}
        ]},
        "vehicles": [{"id": "v1"}],
        "entities": [{"id": "e1"}]
    });
    let ctx = ctx_with(&["something-unrelated"]);
    assert!(
        default_registry()
            .evaluate_with_context(&p, &ctx)
            .iter()
            .all(|f| f.rule_id != "ASSET-RESOLVES"),
        "rows with no asset id carry no reference to resolve: {:?}",
        default_registry().evaluate_with_context(&p, &ctx)
    );
}

#[test]
fn asset_resolves_never_panics_on_empty_or_garbage_with_a_context() {
    let ctx = ctx_with(&["{ABC}Known.et", "prop:ok"]);
    let garbage = [
        json!({}),
        json!(null),
        json!(42),
        json!("a string, not an object"),
        json!([]),
        json!({"editor": 7, "vehicles": 9, "entities": "no"}),
        json!({"vehicles": [null, 5, {"resourceName": 3}, {"id": 1}]}),
        json!({"entities": [null, "x", {"alias": [], "resourceName": {}}]}),
        json!({"editor": {"slots": [null, 9, {"id": 5, "assetId": []}]}}),
        json!({"editor": {"slots": [{"id": "s1", "assetId": "{ABC}Missing.et"}]},
                   "vehicles": [{"id": "v1", "resourceName": "{ABC}Missing.et"}],
                   "entities": [{"id": "e1", "alias": "prop:missing"}]}),
    ];
    let reg = default_registry();
    for p in garbage {
        let _ = reg.evaluate_with_context(&p, &ctx);
        let _ = reg.evaluate(&p);
    }

    assert!(
        reg.evaluate_with_context(&json!({}), &ctx)
            .iter()
            .all(|f| f.rule_id != "ASSET-RESOLVES"),
        "empty payload has no placed assets to flag"
    );
}
