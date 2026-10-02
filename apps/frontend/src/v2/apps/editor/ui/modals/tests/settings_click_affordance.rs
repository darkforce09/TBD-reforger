use super::{
    aggregate_settings, owner_is_routable, row_cursor_class, SettingOwner, OWNER_UNRESOLVED_NOTE,
};
use crate::v2::apps::editor::mission_editor::{route_target, RouteTarget};
use crate::v2::apps::editor::ui::inspector::validation_panel::register_route_probe;
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};
use serde_json::json;

/// Install the probe exactly as `mission_editor`'s mount installs it: the router's own
/// resolution over the document root, asked as a question with no side effect.
///
/// Wave 129 (F7) — the tests below go through this rather than calling `route_target`
/// themselves on the affordance side, because the SHIPPED affordance goes through it. The slot
/// predicate is `false` here for the same reason it is in the editor's own registration for
/// this surface: this aggregation emits `Mission` and `Zone` owners only, which
/// `the_view_emits_no_owner_kind_the_router_cannot_resolve` re-derives rather than trusts.
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

/// **ONE availability decision, not three.** [`owner_is_routable`] asks
/// `validation_panel::subject_id_routes` — an `Rc::clone` of the resolver the click runs — and
/// no live code in this file calls `route_target` itself. The direct call was F7: a third,
/// un-narrowed copy of the decision that the click had already stopped agreeing with.
#[test]
fn the_affordance_asks_the_registered_probe_and_not_the_router_directly() {
    let src = live_code(&super::source::production_source());
    let routable = only_body(&src, &format!("fn owner{}", "_is_routable"));
    assert!(
        routable.contains(&format!("subject_id{}", "_routes")),
        "F7: clickability must be the REGISTERED resolver's answer — the one the click runs — \
         not a second resolution of the same question"
    );
    // NEGATIVE, over the widest haystack the claim is statable in: the whole of this file's
    // LIVE half (the test modules, which legitimately call `route_target` to state the FACT the
    // affordance is checked against, are cut first). Any view or affordance code that resolves
    // the router itself re-opens F7 here.
    assert_eq!(
        src.matches(&format!("route{}", "_target(")).count(),
        0,
        "F7: no live code in this view may resolve the router itself — that is a second \
         availability decision, and the click's is the one that counts"
    );
}

/// The slot predicate the probe is registered with is an ASSUMPTION about what this
/// aggregation emits, so it is checked rather than trusted: the walk mints exactly one entity
/// kind, `Zone`, and every entity row it produces routes. The day a slot-owned (or any other)
/// row appears, this goes red and that predicate must become a real lookup.
#[test]
fn the_view_emits_no_owner_kind_the_router_cannot_resolve() {
    let d = doc();
    let rows = aggregate_settings(&d);
    let mut kinds: Vec<&str> = rows
        .iter()
        .filter_map(|r| match &r.owner {
            SettingOwner::Entity { kind, .. } => Some(*kind),
            SettingOwner::Mission => None,
        })
        .collect();
    kinds.sort_unstable();
    kinds.dedup();
    assert_eq!(
        kinds,
        vec!["Zone"],
        "T-754: the aggregation's entity rows are zones and only zones"
    );
    // Source side: the walk mints ONE `kind:`, so a second entity family cannot slip in without
    // this pin (and the slot predicate the probe is registered with) being revisited.
    let src = live_source(&super::source::production_source());
    let body = only_body(&src, &format!("fn aggregate{}", "_settings"));
    let kind_writes = body.matches(&format!("kind{}", ": \"")).count();
    assert_eq!(
        kind_writes, 1,
        "T-754: the aggregation mints exactly one owner kind today; a second one must be \
         routable before it may be rendered clickable"
    );
    assert!(
        body.contains(&format!("kind{}", ": \"Zone\"")),
        "T-754: and that one kind is Zone — the kind the widened router now resolves"
    );
}

/// One decision, one place. The row styles itself through [`row_cursor_class`] (it spells no
/// pointer/hover class of its own), and the body decides `clickable` by asking
/// [`owner_is_routable`] — which in turn asks the SHIPPED resolution rather than a local table
/// of kinds this file believes are selectable.
///
/// Wave 129 (F7): "the SHIPPED resolution" is now the REGISTERED probe rather than a direct
/// `route_target` call, which is what made the two questions genuinely one.
/// `the_affordance_asks_the_registered_probe_and_not_the_router_directly` owns that half.
#[test]
fn the_affordance_and_the_click_ask_the_same_question() {
    let src = live_code(&super::source::production_source());
    let row = only_body(&src, &format!("fn setting{}", "_row_view"));
    assert!(
        row.contains(&format!("row{}", "_cursor_class(")),
        "T-754: the row must take its cursor/hover classes from the one affordance function"
    );
    let body = only_body(&src, &format!("fn render{}", "_all_settings_body"));
    assert!(
        body.contains(&format!("owner{}", "_is_routable(")),
        "T-754: the body must decide each row's clickability by asking the router"
    );
    let routable = only_body(&src, &format!("fn owner{}", "_is_routable"));
    assert!(
        routable.contains(&format!("validation{}", "_panel"))
            && routable.contains(&format!("subject_id{}", "_routes")),
        "T-754, narrowed by wave 129 (F7): clickability must be the ROUTER's own resolution as \
         the click asks it — the registered probe — not a second opinion about which kinds are \
         selectable, and not a second resolution of the router either"
    );
    // Literals kept: the row must not hand-roll the affordance beside the function that owns it.
    let lit = live_source(&super::source::production_source());
    let row_lit = only_body(&lit, &format!("fn setting{}", "_row_view"));
    assert!(
        !row_lit.contains(&format!("cursor{}", "-pointer")),
        "T-754: a second spelling of the pointer class is how the affordance drifts back out of \
         agreement with the router"
    );
}

/// The unresolved-owner note is now the RESIDUE (the race between the list being built and the
/// click landing), not a standing confession that zones cannot be selected.
#[test]
fn the_unresolved_note_no_longer_claims_zones_are_unroutable() {
    let n = OWNER_UNRESOLVED_NOTE.to_lowercase();
    assert!(
        !n.contains("resolves slots and vehicles"),
        "T-754: the router resolves zones now — the copy must not still say it does not"
    );
    assert!(
        n.contains("deleted"),
        "T-754: the note must name what is actually left — the owner going away underneath the \
         open list"
    );
}
