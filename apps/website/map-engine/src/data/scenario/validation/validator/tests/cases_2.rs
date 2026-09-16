//! Role: Domain regression cases.
//! Position: `mission/validation/validator/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn asset_resolves_is_registered_and_self_check_passes_with_the_context_extension() {
    let reg = default_registry();
    assert!(
        reg.rules().iter().any(|r| r.id() == "ASSET-RESOLVES"),
        "registry missing ASSET-RESOLVES"
    );

    reg.self_check()
        .expect("every rule (incl. ASSET-RESOLVES) must fire on its trip fixture + context");
    reg.assert_self_check();
}

#[test]
fn self_check_catches_a_context_rule_that_ships_without_its_trip_context() {
    let ctx_rule_no_trip = Rule {
        id: "CTX-NO-TRIP",
        severity: Severity::Error,
        primitive: Primitive::PerObjectInvariant,
        applies: |_p, ctx| ctx.known_asset_ids.is_some(),
        eval: |rule, _p, _c| vec![rule.finding("x".into(), "/x".into())],
        trip_fixture: || json!({}),
        trip_context: no_trip_context,
    };
    let err = Registry::new(vec![ctx_rule_no_trip])
        .self_check()
        .expect_err("a context rule with no trip_context must fail self-check");
    assert_eq!(err[0].rule_id, "CTX-NO-TRIP");
    assert!(err[0].reason.contains("`applies` gate"), "{:?}", err[0]);
}

#[test]
fn evaluate_default_context_matches_the_pre_t658_behaviour() {
    let p = json!({
        "schemaVersion": "bad",
        "map": {"terrain": "everon"},
        "editor": {
            "factions": [{"key": "BLUFOR", "name": "US", "squadIds": ["sq1"]}],
            "squads": [{"id": "sq1", "callsign": "Alpha", "name": "A", "slotIds": []}],
            "slots": [{"id": "s1", "position": {"x": 99999.0, "y": 1.0},
                       "assetId": "{ABC}WouldBeUnknown.et"}]
        }
    });
    let via_free_fn = validate_editor_payload(&p);
    let via_default_ctx = default_registry().evaluate_with_context(&p, &EvalContext::default());
    assert_eq!(
        via_free_fn, via_default_ctx,
        "evaluate() and evaluate_with_context(default) must agree"
    );

    assert!(
        via_free_fn.iter().all(|f| f.rule_id != "ASSET-RESOLVES"),
        "ASSET-RESOLVES must stay inert without a catalogue: {via_free_fn:?}"
    );

    for want in ["V3-SLOT-IN-BOUNDS", "V4-SCHEMA-VERSION"] {
        assert!(
            via_free_fn.iter().any(|f| f.rule_id == want),
            "{want} must still fire on the default path: {via_free_fn:?}"
        );
    }
}

#[test]
fn loadout_has_uniform_fires_when_a_loadout_has_no_jacket() {
    let findings = validate_editor_payload(&rule_loadout_has_uniform().trip_fixture());
    let f = finding_for(&findings, "LOADOUT-HAS-UNIFORM");
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.primitive, Primitive::PerObjectInvariant);
    assert_eq!(f.subject, "/editor/slots/0/loadout/wear/jacket");
    assert_eq!(f.subject_id.as_deref(), Some("s1"));
    assert!(f.message.contains("no uniform"), "{f:?}");
}

#[test]
fn loadout_has_uniform_passes_when_a_jacket_is_worn() {
    let p = json!({"editor": {"slots": [
        {"id": "s1", "role": "RFL", "loadout": {"version": 2,
            "wear": {"jacket": "{A}Uniform.et", "vest": "{A}Vest.et"}, "weapons": []}}
    ]}});
    assert!(
        validate_editor_payload(&p)
            .iter()
            .all(|f| f.rule_id != "LOADOUT-HAS-UNIFORM"),
        "a worn uniform must pass: {:?}",
        validate_editor_payload(&p)
    );
}

#[test]
fn loadout_rules_are_conditional_on_a_declared_loadout() {
    for p in [
        json!({}),
        json!({"editor": {"slots": [{"id": "s1", "role": "RFL"}]}}),
    ] {
        for rule in [rule_loadout_has_uniform(), rule_loadout_has_vest()] {
            assert!(
                !rule.applies(&p, &EvalContext::default()),
                "{} must not apply without an authored loadout: {p}",
                rule.id()
            );
        }
        for id in ["LOADOUT-HAS-UNIFORM", "LOADOUT-HAS-VEST"] {
            assert!(
                validate_editor_payload(&p).iter().all(|f| f.rule_id != id),
                "{id} must stay silent without a loadout: {p}"
            );
        }
    }
}

#[test]
fn loadout_has_vest_fires_when_neither_vest_nor_armored_vest_is_worn() {
    let findings = validate_editor_payload(&rule_loadout_has_vest().trip_fixture());
    let f = finding_for(&findings, "LOADOUT-HAS-VEST");
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.primitive, Primitive::PerObjectInvariant);
    assert_eq!(f.subject, "/editor/slots/0/loadout/wear/vest");
    assert_eq!(f.subject_id.as_deref(), Some("s1"));
    assert!(f.message.contains("no vest"), "{f:?}");
}

#[test]
fn loadout_has_vest_accepts_either_vest_or_armored_vest() {
    for wear in [
        json!({"jacket": "{A}U.et", "vest": "{A}ChestRig.et"}),
        json!({"jacket": "{A}U.et", "armoredVest": "{A}PlateCarrier.et"}),
    ] {
        let p = json!({"editor": {"slots": [
            {"id": "s1", "role": "RFL", "loadout": {"version": 2, "wear": wear, "weapons": []}}
        ]}});
        assert!(
            validate_editor_payload(&p)
                .iter()
                .all(|f| f.rule_id != "LOADOUT-HAS-VEST"),
            "either vest garment must satisfy: {p}"
        );
    }
}

#[test]
fn loadout_mag_count_fires_below_the_policy_floor() {
    let rule = rule_loadout_mag_count();
    let ctx = rule.trip_context().expect("declares a trip_context");
    let findings = default_registry().evaluate_with_context(&rule.trip_fixture(), &ctx);
    let f = finding_for(&findings, "LOADOUT-MAG-COUNT");
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.primitive, Primitive::PerObjectInvariant);
    assert_eq!(f.subject, "/editor/slots/0/loadout");
    assert_eq!(f.subject_id.as_deref(), Some("s1"));
    assert!(f.message.contains("1 magazine"), "{f:?}");
    assert!(f.message.contains("at least 3"), "{f:?}");
}

#[test]
fn loadout_mag_count_skips_when_the_policy_carries_no_floor() {
    let p = json!({"editor": {"slots": [
        {"id": "s1", "role": "RFL", "loadout": {"version": 2, "wear": {},
            "weapons": [{"slotIndex": 0, "slotType": "primary", "weapon": "{A}R.et", "magazine": "{A}M.et"}],
            "cargo": []}}
    ]}});

    assert!(
        !rule_loadout_mag_count().applies(&p, &EvalContext::default()),
        "must not apply without a policy floor"
    );
    assert!(
        validate_editor_payload(&p)
            .iter()
            .all(|f| f.rule_id != "LOADOUT-MAG-COUNT"),
        "default ctx must not fire LOADOUT-MAG-COUNT"
    );

    let ctx_empty_policy = EvalContext::default().with_loadout_policy(LoadoutPolicy::default());
    assert!(
        !rule_loadout_mag_count().applies(&p, &ctx_empty_policy),
        "a policy with no magazine floor must not apply"
    );
}

#[test]
fn loadout_mag_count_counts_loaded_mag_plus_same_type_spares_and_respects_the_boundary() {
    let p = json!({"editor": {"slots": [
        {"id": "s1", "role": "RFL", "loadout": {"version": 2, "wear": {},
            "weapons": [{"slotIndex": 0, "slotType": "primary", "weapon": "{A}R.et", "magazine": "{A}M.et"}],
            "cargo": [{"container": "vest", "item": "{A}M.et", "qty": 2}]}}
    ]}});
    let ctx = ctx_min_mags(3);
    assert!(
        default_registry()
            .evaluate_with_context(&p, &ctx)
            .iter()
            .all(|f| f.rule_id != "LOADOUT-MAG-COUNT"),
        "3 mags meets a floor of 3 (boundary): {:?}",
        default_registry().evaluate_with_context(&p, &ctx)
    );

    let p2 = json!({"editor": {"slots": [
        {"id": "s1", "role": "RFL", "loadout": {"version": 2, "wear": {},
            "weapons": [{"slotIndex": 0, "slotType": "primary", "weapon": "{A}R.et", "magazine": "{A}M.et"}],
            "cargo": [{"container": "vest", "item": "{A}Bandage.et", "qty": 9}]}}
    ]}});
    assert!(
        default_registry()
            .evaluate_with_context(&p2, &ctx_min_mags(2))
            .iter()
            .any(|f| f.rule_id == "LOADOUT-MAG-COUNT"),
        "an arbitrary cargo item is not a magazine — one loaded mag is below a floor of 2"
    );
}

#[test]
fn loadout_mag_count_ignores_a_slot_with_no_primary_weapon() {
    let p = json!({"editor": {"slots": [
        {"id": "s1", "role": "MED", "loadout": {"version": 2,
            "wear": {"jacket": "{A}U.et", "vest": "{A}V.et"},
            "weapons": [{"slotIndex": 2, "slotType": "secondary", "weapon": "{A}Pistol.et"}],
            "cargo": []}}
    ]}});
    assert!(
        default_registry()
            .evaluate_with_context(&p, &ctx_min_mags(3))
            .iter()
            .all(|f| f.rule_id != "LOADOUT-MAG-COUNT"),
        "a primary-less slot has no basic load to be below: {:?}",
        default_registry().evaluate_with_context(&p, &ctx_min_mags(3))
    );
}

#[test]
fn loadout_has_equipment_fires_on_missing_required_kinds_over_the_forward_shape() {
    let rule = rule_loadout_has_equipment();
    let ctx = rule.trip_context().expect("declares a trip_context");
    let findings = default_registry().evaluate_with_context(&rule.trip_fixture(), &ctx);
    let f = finding_for(&findings, "LOADOUT-HAS-EQUIPMENT");
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.primitive, Primitive::PerObjectInvariant);
    assert_eq!(f.subject, "/editor/slots/0/loadout/equipment");
    assert_eq!(f.subject_id.as_deref(), Some("s1"));

    assert!(f.message.contains("compass, radio"), "{f:?}");
}

#[test]
fn loadout_has_equipment_is_inert_on_real_data_today_no_policy_no_equipment() {
    let p = json!({"editor": {"slots": [
        {"id": "s1", "role": "RFL", "loadout": {"version": 2,
            "wear": {"jacket": "{A}U.et", "vest": "{A}V.et"}, "weapons": []}}
    ]}});
    assert!(
        !rule_loadout_has_equipment().applies(&p, &EvalContext::default()),
        "no policy ⇒ inert on today's data"
    );
    assert!(
        validate_editor_payload(&p)
            .iter()
            .all(|f| f.rule_id != "LOADOUT-HAS-EQUIPMENT"),
        "must stay silent on real data with no policy"
    );

    let ctx_empty = EvalContext::default()
        .with_loadout_policy(LoadoutPolicy::default().with_required_equipment(HashSet::new()));
    assert!(
        !rule_loadout_has_equipment().applies(&p, &ctx_empty),
        "an empty required-equipment set demands nothing"
    );
}

#[test]
fn loadout_has_equipment_passes_when_the_forward_shape_carries_every_required_kind() {
    let p = json!({"editor": {"slots": [
        {"id": "s1", "role": "RFL", "loadout": {"version": 2, "wear": {}, "weapons": [],
            "equipment": {"map": "{A}Map.et", "compass": "{A}Compass.et", "radio": "{A}Radio.et"}}}
    ]}});
    let kinds: HashSet<String> = ["map", "compass", "radio"]
        .into_iter()
        .map(str::to_string)
        .collect();
    let ctx = EvalContext::default()
        .with_loadout_policy(LoadoutPolicy::default().with_required_equipment(kinds));
    assert!(
        default_registry()
            .evaluate_with_context(&p, &ctx)
            .iter()
            .all(|f| f.rule_id != "LOADOUT-HAS-EQUIPMENT"),
        "full equipment coverage must pass: {:?}",
        default_registry().evaluate_with_context(&p, &ctx)
    );
}

#[test]
fn vehicle_cargo_policy_fires_over_the_ceiling() {
    let rule = rule_vehicle_cargo_policy();
    let ctx = rule.trip_context().expect("declares a trip_context");
    let findings = default_registry().evaluate_with_context(&rule.trip_fixture(), &ctx);
    let f = finding_for(&findings, "VEHICLE-CARGO-POLICY");
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.primitive, Primitive::PerObjectInvariant);
    assert_eq!(f.subject, "/vehicles/0/cargo");
    assert_eq!(f.subject_id.as_deref(), Some("v1"));
    assert!(f.message.contains("35 cargo item"), "{f:?}");
    assert!(f.message.contains("ceiling of 10"), "{f:?}");
    assert!(f.message.contains("fairness"), "{f:?}");
}

#[test]
fn vehicle_cargo_policy_skips_without_a_ceiling_and_respects_the_boundary() {
    let p = json!({"vehicles": [
        {"id": "v1", "resourceName": "{A}Truck.et", "cargo": [{"item": "{A}M.et", "qty": 10}]}
    ]});

    assert!(
        !rule_vehicle_cargo_policy().applies(&p, &EvalContext::default()),
        "no ceiling ⇒ must not apply"
    );

    let ctx = EvalContext::default()
        .with_loadout_policy(LoadoutPolicy::default().with_max_vehicle_cargo_items(10));
    assert!(
        default_registry()
            .evaluate_with_context(&p, &ctx)
            .iter()
            .all(|f| f.rule_id != "VEHICLE-CARGO-POLICY"),
        "10 items at a ceiling of 10 is the boundary, not over it: {:?}",
        default_registry().evaluate_with_context(&p, &ctx)
    );
}

#[test]
fn vehicle_cargo_policy_reports_each_over_vehicle_never_early_exits() {
    let p = json!({"vehicles": [
        {"id": "a", "resourceName": "{A}T.et", "cargo": [{"item": "{A}M.et", "qty": 50}]},
        {"id": "b", "resourceName": "{A}T.et", "cargo": [{"item": "{A}M.et", "qty": 2}]},
        {"id": "c", "resourceName": "{A}T.et", "cargo": [{"item": "{A}M.et", "qty": 99}]}
    ]});
    let ctx = EvalContext::default()
        .with_loadout_policy(LoadoutPolicy::default().with_max_vehicle_cargo_items(10));
    let all = default_registry().evaluate_with_context(&p, &ctx);
    let offenders: Vec<&Finding> = all
        .iter()
        .filter(|f| f.rule_id == "VEHICLE-CARGO-POLICY")
        .collect();
    assert_eq!(offenders.len(), 2, "{offenders:?}");
    assert_eq!(offenders[0].subject_id.as_deref(), Some("a"));
    assert_eq!(offenders[1].subject_id.as_deref(), Some("c"));
}

#[test]
fn cargo_over_capacity_fires_over_a_garment_maximum() {
    let rule = rule_cargo_over_capacity();
    let ctx = rule.trip_context().expect("declares a trip_context");
    let findings = default_registry().evaluate_with_context(&rule.trip_fixture(), &ctx);
    let f = finding_for(&findings, "CARGO-OVER-CAPACITY");
    assert_eq!(f.severity, Severity::Error);
    assert_eq!(f.primitive, Primitive::PerObjectInvariant);
    assert_eq!(f.subject, "/editor/slots/0/loadout/wear/vest");
    assert_eq!(f.subject_id.as_deref(), Some("s1"));
    assert!(f.message.contains("240 / 200 cm³"), "{f:?}");
    assert!(f.message.contains("Plate Carrier"), "{f:?}");
}

#[test]
fn cargo_over_capacity_skips_without_a_catalogue() {
    let p = json!({"editor": {"slots": [
        {"id": "s1", "role": "RFL", "loadout": {"version": 2, "wear": {"vest": "vest_rn"},
            "weapons": [], "cargo": [{"container": "vest", "item": "mag", "qty": 40}]}}
    ]}});
    assert!(
        !rule_cargo_over_capacity().applies(&p, &EvalContext::default()),
        "no catalogue ⇒ must not apply"
    );
    assert!(
        validate_editor_payload(&p)
            .iter()
            .all(|f| f.rule_id != "CARGO-OVER-CAPACITY"),
        "default ctx must not fire CARGO-OVER-CAPACITY"
    );
}

#[test]
fn cargo_over_capacity_agrees_with_the_standalone_scanner() {
    let cat = cargo_catalog();
    let over = json!({"editor": {"slots": [
        {"id": "s1", "role": "RFL", "loadout": {"version": 2, "wear": {"vest": "vest_rn"},
            "weapons": [], "cargo": [{"container": "vest", "item": "mag", "qty": 4}]}}
    ]}});
    let scanner_lines = crate::data::scenario::wire_safety::scan_cargo_capacity(&over, &cat);
    assert_eq!(scanner_lines.len(), 1, "scanner: {scanner_lines:?}");
    let ctx = EvalContext::default().with_cargo_phys(cat.clone());
    let all = default_registry().evaluate_with_context(&over, &ctx);
    let rf: Vec<&Finding> = all
        .iter()
        .filter(|f| f.rule_id == "CARGO-OVER-CAPACITY")
        .collect();
    assert_eq!(rf.len(), 1, "rule: {rf:?}");
    assert_eq!(
        rf[0].message, scanner_lines[0],
        "rule message must be the scanner's line"
    );

    let under = json!({"editor": {"slots": [
        {"id": "s1", "role": "RFL", "loadout": {"version": 2, "wear": {"vest": "vest_rn"},
            "weapons": [], "cargo": [{"container": "vest", "item": "mag", "qty": 3}]}}
    ]}});
    assert!(crate::data::scenario::wire_safety::scan_cargo_capacity(&under, &cat).is_empty());
    assert!(
        default_registry()
            .evaluate_with_context(&under, &ctx)
            .iter()
            .all(|f| f.rule_id != "CARGO-OVER-CAPACITY"),
        "under capacity must be green in the rule too"
    );
}

#[test]
fn loadout_rules_never_panic_on_garbage() {
    let ctx = EvalContext::default()
        .with_cargo_phys(cargo_catalog())
        .with_loadout_policy(
            LoadoutPolicy::default()
                .with_min_magazines(3)
                .with_required_equipment(["map".into()].into_iter().collect())
                .with_max_vehicle_cargo_items(10),
        );
    let garbage = [
        json!({}),
        json!(null),
        json!(42),
        json!("nope"),
        json!([]),
        json!({"editor": {"slots": 7}, "vehicles": "x"}),
        json!({"editor": {"slots": [null, 9, {"id": 5, "loadout": []}]}}),
        json!({"editor": {"slots": [{"id": "s1", "loadout": {"wear": 3, "weapons": 4, "cargo": 5, "equipment": 6}}]}}),
        json!({"editor": {"slots": [{"id": "s1", "loadout": {"wear": {"vest": []},
                "weapons": [null, 3, {"slotIndex": "x", "weapon": [], "magazine": {}}],
                "cargo": [null, {"container": 1, "item": 2, "qty": "no"}],
                "equipment": {"map": 9, "radio": ""}}}]}}),
        json!({"vehicles": [null, 5, {"id": {}, "cargo": [null, {"item": 1, "qty": []}]}]}),
        json!({"editor": {"slots": [{"id": "s1", "loadout": {"wear": {"vest": "vest_rn"},
                "cargo": [{"container": "vest", "item": "mag", "qty": 4}]}}]}}),
    ];
    let reg = default_registry();
    for p in garbage {
        let _ = reg.evaluate_with_context(&p, &ctx);
        let _ = reg.evaluate(&p);
        let _ = validate_editor_payload(&p);
    }
}

#[test]
fn t660_rules_are_registered_and_self_check_passes() {
    let reg = default_registry();
    let ids: Vec<&str> = reg.rules().iter().map(Rule::id).collect();
    for want in [
        "LOADOUT-HAS-UNIFORM",
        "LOADOUT-HAS-VEST",
        "LOADOUT-MAG-COUNT",
        "LOADOUT-HAS-EQUIPMENT",
        "VEHICLE-CARGO-POLICY",
        "CARGO-OVER-CAPACITY",
    ] {
        assert!(ids.contains(&want), "registry missing {want}");
    }

    reg.self_check()
        .expect("every rule (incl. T-660) must fire on its trip fixture + context");
    reg.assert_self_check();
}

#[test]
fn perturb_and_restore_fires_exactly_the_vest_rule() {
    let clean = json!({"editor": {"slots": [
        {"id": "s1", "role": "RFL", "loadout": {"version": 2,
            "wear": {"jacket": "{A}U.et", "vest": "{A}V.et"},
            "weapons": [{"slotIndex": 0, "slotType": "primary", "weapon": "{A}R.et", "magazine": "{A}M.et"}],
            "cargo": []}}
    ]}});

    assert!(
        validate_editor_payload(&clean)
            .iter()
            .all(|f| { !f.rule_id.starts_with("LOADOUT-") && f.rule_id != "CARGO-OVER-CAPACITY" }),
        "baseline must be loadout-clean: {:?}",
        validate_editor_payload(&clean)
    );

    let mut broken = clean.clone();
    broken["editor"]["slots"][0]["loadout"]["wear"]
        .as_object_mut()
        .unwrap()
        .remove("vest");
    let findings = validate_editor_payload(&broken);
    let vest: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.rule_id == "LOADOUT-HAS-VEST")
        .collect();
    assert_eq!(vest.len(), 1, "exactly one vest finding: {findings:?}");
    assert_eq!(vest[0].subject_id.as_deref(), Some("s1"));

    for other in [
        "LOADOUT-HAS-UNIFORM",
        "LOADOUT-MAG-COUNT",
        "LOADOUT-HAS-EQUIPMENT",
        "VEHICLE-CARGO-POLICY",
        "CARGO-OVER-CAPACITY",
    ] {
        assert!(
            findings.iter().all(|f| f.rule_id != other),
            "only the vest rule should fire; {other} also fired: {findings:?}"
        );
    }

    assert!(
        validate_editor_payload(&clean)
            .iter()
            .all(|f| { !f.rule_id.starts_with("LOADOUT-") && f.rule_id != "CARGO-OVER-CAPACITY" }),
        "restoring must return to loadout-clean"
    );
}
