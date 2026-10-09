use super::{SettingOwner, owner_is_routable, row_cursor_class};
use crate::ui::inspector::validation_panel::register_route_probe;
use mission_editing_session::routing::{RouteTarget, route_target};
use serde_json::json;

/// Install the probe exactly as `mission_editor`'s mount installs it: the router's own
/// resolution over the document root, asked as a question with no side effect.
fn install_probe(root: serde_json::Value) {
    register_route_probe(std::rc::Rc::new(move |id: &str| {
        route_target(&root, id, &|_| false).is_some()
    }));
}

/// A mission with a mission-level setting and three zones: a circle, a polygon, and one that was
/// never given a shape (a draw that never committed). The third is the row a click genuinely
/// cannot centre on, so it is what stops the "iff" from being vacuously true.
fn doc() -> serde_json::Value {
    json!({
        "meta": { "terrain": "everon", "environment": { "time": "06:00" } },
        "zonesById": {
            "z-circle": {
                "type": "boundary", "label": "Play area",
                "shape": { "circle": { "x": 100.0, "z": 250.0, "r": 500.0 } },
                "rules": { "graceSeconds": 10 }
            },
            "z-poly": {
                "type": "objective_capture", "label": "Hilltop",
                "shape": { "polygon": [[0.0, 0.0], [10.0, 0.0], [10.0, 20.0], [0.0, 20.0]] },
                "rules": { "captureSeconds": 240 }
            },
            "z-shapeless": { "type": "spawn", "rules": { "penalty": "kill" } }
        }
    })
}

/// **The widening itself.** Every zone id this view can own now resolves through the SHIPPED
/// router's resolution — to the zone's geometric centre, so the click centres like every other
/// click-to-select. Before T-754 all four of these were `None` (the router read the slot SoA and
/// `vehiclesById` and nothing else), which is why 100% of the entity rows were dead.
#[test]
fn the_widened_router_resolves_the_zones_this_view_owns() {
    let d = doc();
    let no_slots = |_: &str| false;
    assert_eq!(
        route_target(&d, "z-circle", &no_slots),
        Some(RouteTarget::Zone { x: 100.0, y: 250.0 }),
        "T-754: a circle zone resolves to its centre (world y IS the document's z)"
    );
    assert_eq!(
        route_target(&d, "z-poly", &no_slots),
        Some(RouteTarget::Zone { x: 5.0, y: 10.0 }),
        "T-754: a polygon zone resolves to its vertex mean"
    );
    assert_eq!(
        route_target(&d, "z-shapeless", &no_slots),
        None,
        "T-754: a zone with no committed shape has nowhere to centre — it must NOT resolve"
    );
    assert_eq!(
        route_target(&d, "z-deleted", &no_slots),
        None,
        "T-754: an id that is no longer in the document resolves to nothing"
    );
}

/// **THE pin the ticket asks for: the affordance is true row by row.**
///
/// `clickable` and `looks clickable` are read from the two ends — [`owner_is_routable`] → the
/// class the row actually wears, versus the router's own resolution — and must agree for every
/// owner, in both directions. A dead click dressed as an affordance fails here.
///
/// Wave 129 (F7): the affordance end now runs through the REGISTERED probe (installed here as
/// the editor installs it), which is the same resolver the click uses. The correspondence being
/// pinned is unchanged; what changed is that there is no longer a second copy of it to drift.
#[test]
fn a_row_is_clickable_iff_the_router_resolves_its_subject() {
    let d = doc();
    install_probe(d.clone());
    let owners = [
        SettingOwner::Mission,
        SettingOwner::Entity {
            kind: "Zone",
            id: "z-circle".into(),
            label: "Play area".into(),
        },
        SettingOwner::Entity {
            kind: "Zone",
            id: "z-poly".into(),
            label: "Hilltop".into(),
        },
        SettingOwner::Entity {
            kind: "Zone",
            id: "z-shapeless".into(),
            label: "Spawn".into(),
        },
        SettingOwner::Entity {
            kind: "Zone",
            id: "z-deleted".into(),
            label: "Gone".into(),
        },
        // A kind no selection surface owns — the shape of the NEXT setting-bearing entity this
        // view absorbs. It must render inert, not hopeful.
        SettingOwner::Entity {
            kind: "Composition",
            id: "comp-1".into(),
            label: "FOB".into(),
        },
    ];
    let (mut clickable_seen, mut inert_seen) = (0usize, 0usize);
    for owner in &owners {
        let wears_pointer = row_cursor_class(owner_is_routable(owner)).contains("cursor-pointer");
        let resolves = owner
            .subject_id()
            .is_some_and(|id| route_target(&d, id, &|_| false).is_some());
        assert_eq!(
            wears_pointer,
            resolves,
            "T-754: `{}` wears the click affordance = {wears_pointer}, but the router resolves \
             it = {resolves}. A row must look clickable IFF clicking it selects something — the \
             wave-115 MAJOR was exactly this pair disagreeing.",
            owner.label()
        );
        if resolves {
            clickable_seen += 1;
        } else {
            inert_seen += 1;
        }
    }
    // Not vacuous: the fixture exercised BOTH sides of the iff.
    assert!(
        clickable_seen >= 2 && inert_seen >= 3,
        "T-754: this pin is only worth anything if it saw both clickable and inert rows \
         (saw {clickable_seen} / {inert_seen})"
    );
    // And the affordance itself is the hover styling, not a name: the "yes" class carries the
    // pointer AND the hover fill, the "no" class carries neither.
    let yes = row_cursor_class(true);
    let no = row_cursor_class(false);
    assert!(
        yes.contains("cursor-pointer") && yes.contains("hover:"),
        "T-754: a clickable row must actually LOOK clickable"
    );
    assert!(
        !no.contains("cursor-pointer") && !no.contains("hover:"),
        "T-754: an unroutable row must wear neither cursor-pointer nor a hover state"
    );
}

/// **Wave 129 (F7) — the row follows the PROBE, and goes inert when the probe says no.**
///
/// T-754 removed the dead click for the mounted case only. Its affordance asked
/// `mission_editor::route_target` over the document root directly — a THIRD copy of "can this
/// subject be clicked", and the one copy F6 never narrowed. The click runs the REGISTERED
/// resolver, which since F6 refuses a `RouteTarget::Zone` while the Zones panel is unmounted.
/// So: open the aggregated-settings dialog (it deliberately survives Backspace hide-chrome),
/// press Backspace to unmount `DockRight`, click a zone-owned row → `cursor-pointer`, nothing
/// happens. The document is IDENTICAL across the two phases below; only the probe's answer
/// differs, which is exactly why the affordance must not be decided from the document.
///
/// The "no probe at all" case (host build, before the editor mounts) is the same `false`, and
/// it is `subject_id_routes`'s own answer — pinned at the seam by `validation_panel`'s
/// `a_seam_with_nothing_installed_reports_failure`. This module reaches it the way its peer
/// `w129_the_panel_asks_the_router` does, with a probe that resolves nothing, so the pin does
/// not depend on which order the harness ran the other tests on this thread.
#[test]
fn a_zone_row_is_inert_when_the_probe_says_no() {
    let d = doc();
    let zone = SettingOwner::Entity {
        kind: "Zone",
        id: "z-circle".into(),
        label: "Play area".into(),
    };
    let pointer = format!("cursor{}", "-pointer");

    // MOUNTED — the probe resolves this zone, so the row wears the affordance.
    install_probe(d.clone());
    let mounted_clickable = row_cursor_class(owner_is_routable(&zone)).contains(&pointer);
    assert!(
        mounted_clickable,
        "F7: with the Zones panel mounted the probe resolves `z-circle`, so its row must be \
         clickable — otherwise this pin's other half proves nothing"
    );

    // UNMOUNTED (Backspace), or the host build, or before the editor mounts — F6's narrowed
    // resolver answers `false`, and the row must render INERT rather than hopeful.
    register_route_probe(std::rc::Rc::new(|_: &str| false));
    let unmounted_inert = !row_cursor_class(owner_is_routable(&zone)).contains(&pointer);
    assert!(
        !owner_is_routable(&zone),
        "F7: the resolver refuses this subject, so the row is not clickable — a fallback to \
         `route_target` here IS the dead click"
    );
    assert!(
        unmounted_inert,
        "F7: a row the resolver refuses must wear no pointer. It kept `cursor-pointer` for the \
         whole of T-754 because the affordance asked the document instead of the resolver"
    );
    assert!(
        !row_cursor_class(owner_is_routable(&zone)).contains("hover:"),
        "F7: nor a hover state — the affordance is the styling, not the name"
    );

    // The document did not move: `route_target` still resolves this zone in BOTH phases. That
    // is the whole finding — the old direct call could not tell the two phases apart.
    assert_eq!(
        route_target(&d, "z-circle", &|_| false),
        Some(RouteTarget::Zone { x: 100.0, y: 250.0 }),
        "F7: the document resolves `z-circle` throughout; only the mount state changed"
    );

    // Not vacuous: this pin saw the affordance BOTH on and off.
    assert!(
        mounted_clickable && unmounted_inert,
        "F7: this pin is only worth anything if it saw a clickable row AND an inert one \
         (saw {mounted_clickable} / {unmounted_inert})"
    );
}
