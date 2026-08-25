use super::{route_target, RouteTarget};
use crate::editor::arsenal::class_r_scrub::live_code;
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

/// The wave-129 wiring: the panel ASKS, and asks the SAME resolution the click acts on.
///
/// The strong form is the count — `route_target` is called ONCE in the whole editor body, and
/// both the probe and the click read that one `resolve`. A second resolution is how the
/// affordance and the click drift apart, which is the entire defect class.
#[test]
fn the_affordance_probe_and_the_click_share_one_resolution() {
    let raw = include_str!("../mission_editor.rs");
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let ed = live_code(&raw[raw.find(anchor.as_str()).expect("the page component")..]);
    assert_eq!(
        ed.matches(&format!("register{}", "_route_probe(")).count(),
        1,
        "wave 129: the panel's affordance seam must be registered exactly once"
    );
    assert_eq!(
        ed.matches(&format!("route{}", "_target(")).count(),
        1,
        "wave 129: ONE resolution in the editor body — the probe and the click must both read \
         it, not each ask their own question"
    );
    assert!(
        ed.contains(&format!("Rc::clone(&resolve{}", ")")),
        "wave 129: the availability narrowing must hold THAT resolution (the same `Rc`), not a \
         second copy of the question"
    );
    assert!(
        ed.contains(&format!("resolve{}", "(subject_id)")),
        "wave 129: and the click acts on what that one resolution returned"
    );
    // F6 — and BOTH seams read the same NARROWED `Rc`, not the raw resolution. See
    // `wave129_f6_probe_and_click_cannot_disagree` for why the narrowing exists.
    assert!(
        ed.contains(&format!("Rc::clone(&available{}", ")")),
        "wave 129 F6: the probe must hold the AVAILABILITY `Rc` — a probe built from the bare \
         resolution says `true` for a zone whose panel is unmounted, which the click answers \
         `false`"
    );
    assert!(
        ed.contains(&format!("available{}", "(subject_id)")),
        "wave 129 F6: and the click must gate on that same availability, so the two cannot \
         answer differently"
    );
    // And the Entity arm rides the selection path, not the Zones-panel path: only `Zone` may
    // divert into `route_select_zone`.
    assert!(
        ed.contains(&format!("matches!(target, RouteTarget::Zone{}", " { .. })")),
        "wave 129: the zone diversion must stay keyed on the Zone arm alone"
    );
}

/// The wiring: the ONE registered router grew a zone arm that drives the Zones panel's own
/// selection seam. No second router, and no zone id smuggled into `select_tool`'s selection.
#[test]
fn the_one_router_routes_zones_through_the_zones_panel() {
    // Anchored at the page component, exactly as the T-655 module does: `cut_test_module` cuts
    // from the FIRST `#[cfg(test)]` to EOF, and this file has one inside `registry_session` long
    // before the mount — scrubbing from the top would leave an empty haystack every pin passes.
    let raw = include_str!("../mission_editor.rs");
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let ed = live_code(&raw[raw.find(anchor.as_str()).expect("the page component")..]);
    assert_eq!(
        ed.matches(&format!("register{}", "_select_by_id(")).count(),
        1,
        "T-754: there must still be exactly ONE registered click-to-select router"
    );
    assert!(
        ed.contains(&format!("route{}", "_target(&root, subject_id")),
        "T-754: the router must resolve through the pure `route_target`, so a view can ask the \
         same question before drawing a click affordance"
    );
    assert!(
        ed.contains(&format!("dock_right::route{}", "_select_zone(subject_id)")),
        "T-754: a zone must be selected through the Zones panel's own selection seam"
    );
}
