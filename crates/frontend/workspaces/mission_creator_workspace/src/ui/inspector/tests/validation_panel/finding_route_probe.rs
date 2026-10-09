//! Validation panel finding route probe tests.

use super::{PanelFinding, finding_is_routable, register_route_probe, row_cursor_class};
use mission_editing_session::routing::route_target;
use mission_validation::Primitive;
use mission_validation::Severity;
use serde_json::json;

fn doc() -> serde_json::Value {
    json!({
        "vehiclesById": { "v1": { "position": { "x": 7.0, "y": 9.0 } } },
        "entitiesById": {
            "e1": {
                "id": "e1",
                "alias": "prop:ammo_crate",
                "position": { "x": 100.0, "y": 200.0, "z": 0.0, "rotation": 90.0 }
            },
            "e-nopos": { "id": "e-nopos", "alias": "prop:x" }
        },
        "zonesById": { "z1": { "shape": { "circle": { "x": 1.0, "z": 2.0, "r": 50.0 } } } }
    })
}

fn is_slot(id: &str) -> bool {
    id == "slot-7"
}

fn pf(rule_id: &str, subject_id: Option<&str>) -> PanelFinding {
    PanelFinding {
        rule_id: rule_id.into(),
        severity: Severity::Error,
        primitive: Primitive::PerObjectInvariant,
        message: format!("{rule_id} says no"),
        subject: format!("/x/{}", subject_id.unwrap_or("none")),
        subject_id: subject_id.map(Into::into),
    }
}

fn install_probe(root: serde_json::Value) {
    register_route_probe(std::rc::Rc::new(move |id: &str| {
        route_target(&root, id, &is_slot).is_some()
    }));
}

#[test]
fn a_finding_row_is_clickable_iff_the_router_resolves_its_subject() {
    let d = doc();
    install_probe(d.clone());
    let rows = [
        pf("ORBAT-SLOT-RESOLVES", Some("slot-7")), // slot
        pf("V-VEHICLE-ASSET", Some("v1")),         // vehicle
        pf("ASSET-RESOLVES", Some("e1")),          // placed object — the wave-129 arm
        pf("ZONE-SHAPE", Some("z1")),              // zone
        pf("ASSET-RESOLVES", Some("e-nopos")),     // object without a position
        pf("ASSET-RESOLVES", Some("e-deleted")),   // stale id (deleted since the last re-eval)
        pf("V2-FACTION-MAX", None),                // positional subject — no offender at all
        pf("X", Some("")),                         // an empty id names nobody
    ];
    let pointer = format!("cursor{}", "-pointer");
    let (mut clickable_seen, mut inert_seen) = (0usize, 0usize);
    for f in &rows {
        let wears_pointer = row_cursor_class(finding_is_routable(f)).contains(&pointer);
        let resolves = f
            .subject_id
            .as_ref()
            .map(|id| id.as_str())
            .is_some_and(|id| !id.is_empty() && route_target(&d, id, &is_slot).is_some());
        assert_eq!(
            wears_pointer, resolves,
            "wave 129: `{}` wears the click affordance = {wears_pointer}, but the router \
                 resolves it = {resolves}. A row must look clickable IFF clicking it selects \
                 something — a dead click dressed as an affordance is the whole defect.",
            f.subject
        );
        if resolves {
            clickable_seen += 1;
        } else {
            inert_seen += 1;
        }
    }
    assert!(
        clickable_seen >= 4 && inert_seen >= 4,
        "wave 129: this pin is only worth anything if it saw both clickable and inert rows \
             (saw {clickable_seen} / {inert_seen})"
    );
    assert!(
        finding_is_routable(&pf("ASSET-RESOLVES", Some("e1"))),
        "wave 129: an ASSET-RESOLVES finding on a placed object must route — the engine emits \
             these today, which is what made this the reachable half of the defect"
    );
    let yes = row_cursor_class(true);
    let no = row_cursor_class(false);
    assert!(
        yes.contains(&pointer) && yes.contains("hover:"),
        "wave 129: a clickable row must actually LOOK clickable"
    );
    assert!(
        !no.contains(&pointer) && !no.contains("hover:"),
        "wave 129: an unroutable row must wear neither cursor-pointer nor a hover state"
    );
}

#[test]
fn with_no_router_registered_every_row_renders_inert() {
    register_route_probe(std::rc::Rc::new(|_: &str| false));
    for f in [
        pf("ASSET-RESOLVES", Some("e1")),
        pf("ORBAT-SLOT-RESOLVES", Some("slot-7")),
        pf("V2-FACTION-MAX", None),
    ] {
        assert!(
            !finding_is_routable(&f),
            "wave 129: with nothing to ask, a row must render inert rather than hopeful"
        );
    }
}
