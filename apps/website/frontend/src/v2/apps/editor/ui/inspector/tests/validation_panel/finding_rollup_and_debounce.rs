use super::*;
use serde_json::json;
use website_map_engine::data::scenario::validate::default_registry;
use website_map_engine::data::scenario::validate::EvalContext;

/// A payload that fires `ORBAT-CALLSIGN-UNIQUE`: BLUFOR with two squads both called "Alpha" (the
/// same shape as the rule's own trip fixture, inlined here since the rule constructor is private
/// to the engine crate). `sq2` is the second row (index 1) and the reported offender.
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

/* ── Rollup counts ── */

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
    // The ticket's example shape: "3 errors · 5 warnings". Worst first; only non-zero appear.
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
    // Exactly one of each present → singular; the absent Warning band is omitted entirely.
    assert_eq!(r.chip_text(), "1 error · 1 info");
}

#[test]
fn empty_rollup_is_empty_and_has_no_chip() {
    let r = Rollup::of(&[]);
    assert!(r.is_empty());
    assert!(!r.has_blocking());
    assert_eq!(r.chip_text(), "");
}

/* ── grouping by rule with counts ── */

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
    // Errors sort ahead of warnings regardless of first-seen order.
    assert_eq!(groups[0].rule_id, "ERR-RULE");
    assert_eq!(groups[0].count(), 2);
    assert_eq!(groups[0].severity, Severity::Error);
    assert_eq!(groups[1].rule_id, "WARN-RULE");
    assert_eq!(groups[1].count(), 3);
}

/* ── click-to-select routing: subject_id → selection call pins ── */

#[test]
fn a_finding_with_a_subject_id_names_an_offender() {
    // Click-to-select routes on `subject_id` (T-657). This is the FACT that the rule kept an
    // offender id — NOT the claim that the row is clickable; that one belongs to the router
    // (`finding_is_routable`), see `w129_the_panel_asks_the_router`.
    let f = pf("ORBAT-SLOT-RESOLVES", Severity::Error, Some("slot-7"));
    assert!(f.is_selectable());
    assert_eq!(f.subject_id.as_deref(), Some("slot-7"));
}

#[test]
fn a_positional_finding_names_no_offender() {
    // V2-FACTION-MAX / V4-SCHEMA-VERSION carry no entity id — their row renders, and renders
    // inert, because there is nothing for the router to resolve.
    let f = pf("V2-FACTION-MAX", Severity::Warning, None);
    assert!(!f.is_selectable());
    let blank = pf("X", Severity::Warning, Some(""));
    assert!(!blank.is_selectable(), "an empty subject_id names nobody");
}

#[test]
fn subject_id_survives_the_flatten_from_an_engine_finding() {
    // The click-to-select KEY must survive `PanelFinding::from_finding` — the panel selects on
    // the flattened row, so if the flatten dropped `subject_id`, click-to-select would be dead.
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
    // The T-655 pointer fix: positional subject, stable id in subject_id (the selection key).
    assert_eq!(callsign.subject, "/editor/squads/1");
    assert_eq!(callsign.subject_id.as_deref(), Some("sq2"));
    assert!(callsign.is_selectable());
}

/* ── debounce behaviour (pure timer logic) ── */

#[test]
fn debounce_fires_once_after_the_trailing_window() {
    let mut d = Debouncer::new(REEVAL_DEBOUNCE_MS);
    d.bump(1000.0);
    // Not yet — the window has not elapsed.
    assert!(!d.should_fire(1000.0 + REEVAL_DEBOUNCE_MS - 1.0));
    // Exactly at the window → due.
    assert!(d.should_fire(1000.0 + REEVAL_DEBOUNCE_MS));
    assert!(d.take_fire());
    // Consumed — a second take with nothing pending is a no-op.
    assert!(!d.take_fire());
    assert!(!d.should_fire(1000.0 + 10_000.0));
}

#[test]
fn debounce_a_burst_collapses_to_one_trailing_fire() {
    // The core contract: many bumps in a burst → ONE evaluation, the window after the LAST bump.
    let mut d = Debouncer::new(REEVAL_DEBOUNCE_MS);
    d.bump(1000.0);
    d.bump(1100.0);
    d.bump(1200.0); // last bump of the burst
                    // A check 250 ms after the FIRST bump must NOT fire — a newer bump reset the window.
    assert!(!d.should_fire(1000.0 + REEVAL_DEBOUNCE_MS));
    // 250 ms after the LAST bump → fires exactly once.
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

/* ── NO SEVERITY ON CORRECT INPUT: a clean payload → an empty panel ── */

#[test]
fn a_clean_payload_produces_an_empty_panel() {
    // The ticket's hard rule, asserted at the panel level: a clean, well-formed mission run
    // through the SAME registry the panel uses yields zero findings, so `Rollup::is_empty()` and
    // the panel shows the quiet empty state — never a severity on correct input.
    //
    // "Clean" per the T-657 tightening: every squad has an identity (callsign) AND a leader; every
    // slot resolves a role AND a squad; ≤4 factions; a valid schemaVersion; slots in bounds.
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
    // Same clean mission but now with a slot that carries an assetId that DOES resolve in the
    // supplied catalogue — the T-658 context path must also produce no findings on correct input.
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

/* ── fire the rollup rule once: perturb → fail → restore (the ticket's fired proof) ── */

#[test]
fn perturbing_a_clean_mission_fires_the_rollup_then_restoring_clears_it() {
    // The ticket asks the rollup be fired once. Start clean (empty rollup), PERTURB the mission
    // into a defect (a second squad sharing the callsign on the same side → ORBAT-CALLSIGN-UNIQUE
    // fires), assert the rollup now counts it, then RESTORE and assert the rollup is empty again.
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

    // Clean baseline: distinct callsigns → empty rollup.
    let clean_rows: Vec<PanelFinding> = default_registry()
        .evaluate(&base_squads(false))
        .iter()
        .map(PanelFinding::from_finding)
        .collect();
    assert!(
        Rollup::of(&clean_rows).is_empty(),
        "baseline must be clean; got {clean_rows:?}"
    );

    // Perturb: duplicate callsign on one side → the rule fires; the rollup counts a warning.
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
    // The rollup chip renders the fired count.
    assert_eq!(dirty_rollup.chip_text(), "1 warning");

    // Restore: back to distinct callsigns → the rollup is empty again.
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

/* ── evaluate_source: the panel's engine call is defensive + threads the catalogue ── */

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
    // On the host (and pre-mount) no payload source is registered, so the panel evaluates to
    // empty rather than panicking — the "native build simply sees None" contract.
    assert!(evaluate_now().is_empty());
}

#[test]
fn click_to_select_is_a_no_op_without_a_registered_router() {
    // The click-to-select seam: with no router registered (host / pre-mount) a finding click is
    // a safe no-op returning false — it never panics and never touches a disposed doc. On wasm
    // the router (installed from `mission_editor.rs`) does the real subject_id → selection route.
    assert!(!route_select_by_subject_id("slot-7"));
    // A registered router IS consulted, and its verdict is returned verbatim (id-shape agnostic).
    register_select_by_id(std::rc::Rc::new(|id: &str| id == "slot-7"));
    assert!(route_select_by_subject_id("slot-7"));
    assert!(!route_select_by_subject_id("slot-other"));
}

/* ── the severity ladder legend is complete ── */

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
    // The tag hook agrees with the engine's spelling.
    assert_eq!(severity_tag(Severity::Error), "error");
    assert_eq!(severity_tag(Severity::Warning), "warning");
    assert_eq!(severity_tag(Severity::Info), "info");
}
