use super::{RouteTarget, route_target};
use serde_json::json;

fn doc() -> serde_json::Value {
    json!({
        "vehiclesById": { "v1": { "position": { "x": 7.0, "y": 9.0 } } },
        // Wave 129 — placed world objects, `add_entity`'s row shape verbatim (`doc/store.rs`).
        "entitiesById": {
            "e1": {
                "id": "e1",
                "alias": "prop:ammo_crate",
                "resourceName": "{FA}Prefabs/Props/AmmoBox.et",
                "position": { "x": 100.0, "y": 200.0, "z": 0.0, "rotation": 90.0 }
            },
            // A row mid-write / hand-authored without a position: nothing to centre on, so the
            // router must resolve NOTHING rather than centring on (0, 0).
            "e-nopos": { "id": "e-nopos", "alias": "prop:x" }
        },
        "zonesById": {
            "z-circle": { "shape": { "circle": { "x": 100.0, "z": 250.0, "r": 500.0 } } },
            "z-poly": { "shape": { "polygon": [[0.0, 0.0], [30.0, 0.0], [0.0, 30.0]] } },
            "z-shapeless": { "type": "spawn" }
        }
    })
}

/// The resolution, arm by arm — including the ORDER, which is the shipped router's order (slot,
/// then vehicle, then the new zone arm), so the widening cannot change what an id already
/// resolved to. `None` still means "select nothing and keep the current selection".
#[test]
fn every_arm_resolves_and_the_order_is_the_shipped_one() {
    let d = doc();
    let no_slots = |_: &str| false;
    assert_eq!(
        route_target(&d, "v1", &no_slots),
        Some(RouteTarget::Vehicle { x: 7.0, y: 9.0 })
    );
    assert_eq!(
        route_target(&d, "z-circle", &no_slots),
        Some(RouteTarget::Zone { x: 100.0, y: 250.0 })
    );
    assert_eq!(
        route_target(&d, "z-poly", &no_slots),
        Some(RouteTarget::Zone { x: 10.0, y: 10.0 })
    );
    assert_eq!(route_target(&d, "z-shapeless", &no_slots), None);
    assert_eq!(route_target(&d, "nobody", &no_slots), None);
    // A slot wins over everything else, exactly as the SoA lookup did when it ran first.
    assert_eq!(
        route_target(&d, "v1", &|_| true),
        Some(RouteTarget::Slot),
        "T-754: the slot arm must still take precedence — the widening reorders nothing"
    );
    // A garbage document resolves nothing rather than panicking inside a click handler.
    assert_eq!(route_target(&json!(null), "z-circle", &no_slots), None);
    assert_eq!(
        route_target(
            &json!({ "zonesById": { "z": { "shape": { "polygon": [] } } } }),
            "z",
            &no_slots
        ),
        None
    );
}

/// **Wave 129 — a placed world object resolves.** The reachable half of the same defect: the
/// engine emits `ASSET-RESOLVES` findings keyed by an `entities[]` row id, and before this arm
/// every one of them resolved to `None` under a `cursor-pointer` row.
///
/// Perturbation RED: delete the `entitiesById` arm from [`route_target`].
#[test]
fn a_placed_object_resolves_at_its_authored_position() {
    let d = doc();
    let no_slots = |_: &str| false;
    assert_eq!(
        route_target(&d, "e1", &no_slots),
        Some(RouteTarget::Entity { x: 100.0, y: 200.0 }),
        "wave 129: a placed object must resolve to its authored position — an ASSET-RESOLVES \
         finding names exactly this id"
    );
    // A row with no position, and a deleted one: nothing to centre on ⇒ nothing to select.
    assert_eq!(route_target(&d, "e-nopos", &no_slots), None);
    assert_eq!(route_target(&d, "e-deleted", &no_slots), None);
    // The widening reorders nothing: a slot still wins, and vehicles/zones still resolve as they
    // did (the by-id maps are keyed by disjoint minted ids, so order cannot matter).
    assert_eq!(route_target(&d, "e1", &|_| true), Some(RouteTarget::Slot));
    assert_eq!(
        route_target(&d, "v1", &no_slots),
        Some(RouteTarget::Vehicle { x: 7.0, y: 9.0 })
    );
    assert_eq!(
        route_target(&d, "z-circle", &no_slots),
        Some(RouteTarget::Zone { x: 100.0, y: 250.0 })
    );
}
