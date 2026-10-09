use super::{RouteTarget, route_availability, route_target};
use serde_json::json;

fn doc() -> serde_json::Value {
    json!({
        "vehiclesById": { "v1": { "position": { "x": 7.0, "y": 9.0 } } },
        "entitiesById": {
            "e1": { "position": { "x": 100.0, "y": 200.0, "z": 0.0, "rotation": 90.0 } }
        },
        "zonesById": {
            "z-circle": { "shape": { "circle": { "x": 100.0, "z": 250.0, "r": 500.0 } } },
            // A draw that was never committed: resolves to nothing on ANY panel state.
            "z-shapeless": { "type": "spawn" }
        }
    })
}

/// `resolve`'s job in the registered block: [`route_target`] plus the centre the click will use.
/// The slot centre comes off the SoA there; any pair does here, the arm is what matters.
fn resolved(
    d: &serde_json::Value,
    subject_id: &str,
    is_slot: bool,
) -> Option<(RouteTarget, f64, f64)> {
    let target = route_target(d, subject_id, &|_| is_slot)?;
    let (x, y) = match target {
        RouteTarget::Slot => (1.0, 2.0),
        RouteTarget::Vehicle { x, y }
        | RouteTarget::Entity { x, y }
        | RouteTarget::Zone { x, y }
        | RouteTarget::Comment { x, y } => (x, y),
    };
    Some((target, x, y))
}

/// **The probe**, as `register_route_probe` computes it: availability, asked.
fn probe_says(d: &serde_json::Value, id: &str, is_slot: bool, zone_panel_live: bool) -> bool {
    route_availability(resolved(d, id, is_slot), &|| zone_panel_live).is_some()
}

/// **The click**, modelled the way `register_select_by_id` is WRITTEN — availability gate, then
/// ACT, and for a `Zone` the act is `eden_dock_right::route_select_zone`, whose returned `bool`
/// is precisely "was the Zones panel there?" (`route_select_zone`: `Some(f) => true, None =>
/// false`). Modelling the seam SEPARATELY from the oracle is what makes this test able to fail:
/// if the narrowing stops consulting panel liveness, the click still does, and the two diverge.
fn click_succeeds(d: &serde_json::Value, id: &str, is_slot: bool, zone_panel_live: bool) -> bool {
    let Some((target, _cx, _cy)) =
        route_availability(resolved(d, id, is_slot), &|| zone_panel_live)
    else {
        return false;
    };
    if matches!(target, RouteTarget::Zone { .. }) {
        return zone_panel_live;
    }
    true
}

/// Every target kind, both panel states: **the probe's answer IS the click's outcome.**
///
/// Perturbation RED: drop the `Zone` narrowing from [`route_availability`] (make it
/// `Some((target, x, y))` unconditionally) — the `zone, Zones panel unmounted` row then reads
/// probe `true` / click `false`, which is the live F6 defect.
#[test]
fn the_probe_answers_the_question_the_click_will_answer_for_every_target_kind() {
    let d = doc();
    // (label, subject_id, is_slot, zone_panel_live, clicking it does something)
    let table: &[(&str, &str, bool, bool, bool)] = &[
        ("slot", "s1", true, true, true),
        // A slot is selected in the editor selection, which lives as long as the router does —
        // hide-chrome must NOT make it inert. This row is why the narrowing is arm-keyed.
        ("slot, chrome hidden", "s1", true, false, true),
        ("vehicle", "v1", false, true, true),
        ("vehicle, chrome hidden", "v1", false, false, true),
        ("entity (placed object)", "e1", false, true, true),
        ("entity, chrome hidden", "e1", false, false, true),
        ("zone, Zones panel mounted", "z-circle", false, true, true),
        // THE F6 ROW.
        (
            "zone, Zones panel unmounted",
            "z-circle",
            false,
            false,
            false,
        ),
        ("zone with no shape", "z-shapeless", false, true, false),
        ("unresolvable id", "nobody", false, true, false),
        (
            "unresolvable id, chrome hidden",
            "nobody",
            false,
            false,
            false,
        ),
    ];
    for &(label, id, is_slot, live, expected) in table {
        let probe = probe_says(&d, id, is_slot, live);
        let click = click_succeeds(&d, id, is_slot, live);
        assert_eq!(
            probe, click,
            "wave 129 F6 [{label}]: the affordance probe and the click disagree — a row painted \
             clickable over a click that does nothing (or an inert row over a live click)"
        );
        assert_eq!(
            probe, expected,
            "wave 129 F6 [{label}]: wrong availability — a row is clickable IFF clicking it \
             does something"
        );
    }
    // The table is not vacuous in either direction: it saw clickable rows AND dead rows.
    assert!(
        table.iter().any(|r| r.4) && table.iter().any(|r| !r.4),
        "wave 129 F6: the table must exercise both outcomes, or `probe == click` is trivially \
         satisfiable"
    );
}
