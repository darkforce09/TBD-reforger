//! Validation panel finding rollup and debounce tests.

use super::*;
use serde_json::json;
use website_map_engine::data::scenario::validate::default_registry;
use website_map_engine::data::scenario::validate::EvalContext;

fn duplicate_callsign_payload() -> serde_json::Value {
    json!({
        "editor": {
            "factions": [{"key": "BLUFOR", "name": "US Army", "squadIds": ["sq1", "sq2"]}],
            "squads": [
                {"id": "sq1", "callsign": "Alpha", "name": "Alpha 1-1", "slotIds": []},
                {"id": "sq2", "callsign": "Alpha", "name": "Alpha 1-2", "slotIds": []}
            ]
        }
    })
}

fn pf(rule_id: &str, severity: Severity, subject_id: Option<&str>) -> PanelFinding {
    PanelFinding {
        rule_id: rule_id.to_string(),
        severity,
        primitive: Primitive::PerObjectInvariant,
        message: format!("{rule_id} says no"),
        subject: format!("/x/{}", subject_id.unwrap_or("none")),
        subject_id: subject_id.map(str::to_string),
    }
}

#[test]
fn rollup_counts_by_severity() {
    let rows = vec![
        pf("A", Severity::Error, Some("s1")),
        pf("B", Severity::Error, Some("s2")),
        pf("C", Severity::Error, Some("s3")),
        pf("D", Severity::Warning, Some("s4")),
        pf("E", Severity::Warning, Some("s5")),
        pf("F", Severity::Info, Some("s6")),
    ];
    let r = Rollup::of(&rows);
    assert_eq!(r.errors, 3);
    assert_eq!(r.warnings, 2);
    assert_eq!(r.infos, 1);
    assert_eq!(r.total(), 6);
    assert!(!r.is_empty());
    assert!(r.has_blocking());
}

#[test]
fn rollup_chip_text_is_the_one_line_summary() {
    let mut rows = Vec::new();
    for i in 0..3 {
        rows.push(pf("E", Severity::Error, Some(&format!("e{i}"))));
    }
    for i in 0..5 {
        rows.push(pf("W", Severity::Warning, Some(&format!("w{i}"))));
    }
    let r = Rollup::of(&rows);
    assert_eq!(r.chip_text(), "3 errors · 5 warnings");
}

#[test]
fn rollup_chip_text_singular_and_omits_zero_severities() {
    let rows = vec![
        pf("E", Severity::Error, Some("e1")),
        pf("I", Severity::Info, Some("i1")),
    ];
    let r = Rollup::of(&rows);
    assert_eq!(r.chip_text(), "1 error · 1 info");
}

#[test]
fn empty_rollup_is_empty_and_has_no_chip() {
    let r = Rollup::of(&[]);
    assert!(r.is_empty());
    assert!(!r.has_blocking());
    assert_eq!(r.chip_text(), "");
}

#[test]
fn group_by_rule_groups_and_counts_worst_first() {
    let rows = vec![
        pf("WARN-RULE", Severity::Warning, Some("s1")),
        pf("ERR-RULE", Severity::Error, Some("s2")),
        pf("WARN-RULE", Severity::Warning, Some("s3")),
        pf("ERR-RULE", Severity::Error, Some("s4")),
        pf("WARN-RULE", Severity::Warning, Some("s5")),
    ];
    let groups = group_by_rule(&rows);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].rule_id, "ERR-RULE");
    assert_eq!(groups[0].count(), 2);
    assert_eq!(groups[0].severity, Severity::Error);
    assert_eq!(groups[1].rule_id, "WARN-RULE");
    assert_eq!(groups[1].count(), 3);
}

#[test]
fn a_finding_with_a_subject_id_names_an_offender() {
    let f = pf("ORBAT-SLOT-RESOLVES", Severity::Error, Some("slot-7"));
    assert!(f.is_selectable());
    assert_eq!(f.subject_id.as_deref(), Some("slot-7"));
}

#[test]
fn a_positional_finding_names_no_offender() {
    let f = pf("V2-FACTION-MAX", Severity::Warning, None);
    assert!(!f.is_selectable());
    let blank = pf("X", Severity::Warning, Some(""));
    assert!(!blank.is_selectable(), "an empty subject_id names nobody");
}

#[test]
fn subject_id_survives_the_flatten_from_an_engine_finding() {
    let payload = duplicate_callsign_payload();
    let engine_findings = default_registry().evaluate(&payload);
    let rows: Vec<PanelFinding> = engine_findings
        .iter()
        .map(PanelFinding::from_finding)
        .collect();
    let callsign = rows
        .iter()
        .find(|r| r.rule_id == "ORBAT-CALLSIGN-UNIQUE")
        .expect("callsign finding present");
    assert_eq!(callsign.subject, "/editor/squads/1");
    assert_eq!(callsign.subject_id.as_deref(), Some("sq2"));
    assert!(callsign.is_selectable());
}

#[test]
fn debounce_fires_once_after_the_trailing_window() {
    let mut d = Debouncer::new(REEVAL_DEBOUNCE_MS);
    d.bump(1000.0);
    assert!(!d.should_fire(1000.0 + REEVAL_DEBOUNCE_MS - 1.0));
    assert!(d.should_fire(1000.0 + REEVAL_DEBOUNCE_MS));
    assert!(d.take_fire());
    assert!(!d.take_fire());
    assert!(!d.should_fire(1000.0 + 10_000.0));
}

#[test]
fn debounce_a_burst_collapses_to_one_trailing_fire() {
    let mut d = Debouncer::new(REEVAL_DEBOUNCE_MS);
    d.bump(1000.0);
    d.bump(1100.0);
    d.bump(1200.0); // last bump of the burst
    assert!(!d.should_fire(1000.0 + REEVAL_DEBOUNCE_MS));
    assert!(d.should_fire(1200.0 + REEVAL_DEBOUNCE_MS));
    assert!(d.take_fire());
    assert!(!d.is_pending());
}

#[test]
fn debounce_is_idle_until_first_bump() {
    let d = Debouncer::new(REEVAL_DEBOUNCE_MS);
    assert!(!d.is_pending());
    assert!(!d.should_fire(1_000_000.0));
}

#[test]
fn a_clean_payload_produces_an_empty_panel() {
    let clean = json!({
        "schemaVersion": 1,
        "map": {"terrain": "everon"},
        "editor": {
            "factions": [{"key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]}],
            "squads": [{
                "id": "sq1", "callsign": "Alpha", "name": "Alpha 1-1",
                "slotIds": ["s1"], "leaderSlotId": "s1"
            }],
            "slots": [{
                "id": "s1", "role": "SL",
                "position": {"x": 6400.0, "y": 6400.0, "z": 0.0}
            }]
        }
    });
    let findings = default_registry().evaluate(&clean);
    let rows: Vec<PanelFinding> = findings.iter().map(PanelFinding::from_finding).collect();
    let rollup = Rollup::of(&rows);
    assert!(
        rollup.is_empty(),
        "a clean payload must produce NO findings (no severity on correct input); got: {rows:?}"
    );
}

#[test]
fn a_clean_payload_stays_clean_with_a_supplied_catalogue() {
    let asset = "{ABC}Prefabs/Characters/Rifleman.et";
    let clean = json!({
        "schemaVersion": 1,
        "map": {"terrain": "everon"},
        "editor": {
            "factions": [{"key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]}],
            "squads": [{
                "id": "sq1", "callsign": "Alpha", "name": "Alpha 1-1",
                "slotIds": ["s1"], "leaderSlotId": "s1"
            }],
            "slots": [{
                "id": "s1", "role": "SL", "assetId": asset,
                "position": {"x": 6400.0, "y": 6400.0, "z": 0.0}
            }]
        }
    });
    let ids: std::collections::HashSet<String> = [asset.to_string()].into_iter().collect();
    let ctx = EvalContext::default().with_known_asset_ids(ids);
    let findings = default_registry().evaluate_with_context(&clean, &ctx);
    assert!(
        findings.is_empty(),
        "a clean payload with a resolvable asset must produce NO findings; got: {findings:?}"
    );
}

#[test]
fn perturbing_a_clean_mission_fires_the_rollup_then_restoring_clears_it() {
    let base_squads = |dup: bool| {
        let second = if dup { "Alpha" } else { "Bravo" };
        json!({
            "schemaVersion": 1,
            "map": {"terrain": "everon"},
            "editor": {
                "factions": [{"key": "BLUFOR", "name": "US", "squadIds": ["sq1", "sq2"]}],
                "squads": [
                    {"id": "sq1", "callsign": "Alpha", "name": "A", "slotIds": ["s1"], "leaderSlotId": "s1"},
                    {"id": "sq2", "callsign": second, "name": "B", "slotIds": ["s2"], "leaderSlotId": "s2"}
                ],
                "slots": [
                    {"id": "s1", "role": "SL", "position": {"x": 6400.0, "y": 6400.0, "z": 0.0}},
                    {"id": "s2", "role": "SL", "position": {"x": 6410.0, "y": 6410.0, "z": 0.0}}
                ]
            }
        })
    };

    let clean_rows: Vec<PanelFinding> = default_registry()
        .evaluate(&base_squads(false))
        .iter()
        .map(PanelFinding::from_finding)
        .collect();
    assert!(
        Rollup::of(&clean_rows).is_empty(),
        "baseline must be clean; got {clean_rows:?}"
    );

    let dirty_rows: Vec<PanelFinding> = default_registry()
        .evaluate(&base_squads(true))
        .iter()
        .map(PanelFinding::from_finding)
        .collect();
    let dirty_rollup = Rollup::of(&dirty_rows);
    assert!(
        !dirty_rollup.is_empty(),
        "perturbed mission must fire a finding"
    );
    assert!(
        dirty_rows
            .iter()
            .any(|r| r.rule_id == "ORBAT-CALLSIGN-UNIQUE"),
        "the perturbation must fire ORBAT-CALLSIGN-UNIQUE; got {dirty_rows:?}"
    );
    assert_eq!(dirty_rollup.warnings, 1);
    assert_eq!(dirty_rollup.chip_text(), "1 warning");

    let restored_rows: Vec<PanelFinding> = default_registry()
        .evaluate(&base_squads(false))
        .iter()
        .map(PanelFinding::from_finding)
        .collect();
    assert!(
        Rollup::of(&restored_rows).is_empty(),
        "restoring must clear the rollup; got {restored_rows:?}"
    );
}

#[test]
fn evaluate_source_runs_the_engine_and_flattens() {
    let source = PayloadSource {
        payload: duplicate_callsign_payload(),
        known_asset_ids: None,
    };
    let rows = evaluate_source(&source);
    assert!(rows.iter().any(|r| r.rule_id == "ORBAT-CALLSIGN-UNIQUE"));
}

#[test]
fn evaluate_now_is_empty_without_a_registered_source() {
    assert!(evaluate_now().is_empty());
}

#[test]
fn click_to_select_is_a_no_op_without_a_registered_router() {
    assert!(!route_select_by_subject_id("slot-7"));
    register_select_by_id(std::rc::Rc::new(|id: &str| id == "slot-7"));
    assert!(route_select_by_subject_id("slot-7"));
    assert!(!route_select_by_subject_id("slot-other"));
}

#[test]
fn the_severity_ladder_covers_every_severity_with_a_meaning() {
    assert_eq!(SEVERITY_LADDER.len(), 3);
    assert_eq!(SEVERITY_LADDER[0].severity, Severity::Error);
    assert_eq!(SEVERITY_LADDER[1].severity, Severity::Warning);
    assert_eq!(SEVERITY_LADDER[2].severity, Severity::Info);
    for rung in SEVERITY_LADDER {
        assert!(!rung.label.is_empty());
        assert!(!rung.meaning.is_empty(), "{rung:?} needs a meaning");
    }
    assert_eq!(severity_tag(Severity::Error), "error");
    assert_eq!(severity_tag(Severity::Warning), "warning");
    assert_eq!(severity_tag(Severity::Info), "info");
}
