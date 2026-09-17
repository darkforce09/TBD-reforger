use super::{finding_is_routable, register_route_probe, row_cursor_class, PanelFinding};
use crate::v2::apps::editor::mission_editor::route_target;
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};
use serde_json::json;
use website_map_engine::data::scenario::validate::Primitive;
use website_map_engine::data::scenario::validate::Severity;

/// A document root in `small_maps_json` shape carrying one row of every kind the router can meet
/// — plus the two shapes it must refuse: an object row with no position, and (by omission) an id
/// that is no longer in the document.
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

/// The one fact the small-maps root cannot answer, supplied here as the editor supplies it.
fn is_slot(id: &str) -> bool {
    id == "slot-7"
}

fn pf(rule_id: &str, subject_id: Option<&str>) -> PanelFinding {
    PanelFinding {
        rule_id: rule_id.to_string(),
        severity: Severity::Error,
        primitive: Primitive::PerObjectInvariant,
        message: format!("{rule_id} says no"),
        subject: format!("/x/{}", subject_id.unwrap_or("none")),
        subject_id: subject_id.map(str::to_string),
    }
}

/// Install the probe exactly as `mission_editor`'s mount installs it: the router's own
/// resolution over the document root, asked as a question.
fn install_probe(root: serde_json::Value) {
    register_route_probe(std::rc::Rc::new(move |id: &str| {
        route_target(&root, id, &is_slot).is_some()
    }));
}

/// **THE pin: the affordance is true row by row.** What the row wears versus what the router
/// resolves, for every subject kind, in both directions.
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
            .as_deref()
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
    // Not vacuous: the fixture exercised BOTH sides of the iff.
    assert!(
        clickable_seen >= 4 && inert_seen >= 4,
        "wave 129: this pin is only worth anything if it saw both clickable and inert rows \
             (saw {clickable_seen} / {inert_seen})"
    );
    // The reachable case, named: a placed-object finding — the row the verifier caught wearing a
    // pointer over nothing — is clickable now, and clickable BECAUSE the router resolves it.
    assert!(
        finding_is_routable(&pf("ASSET-RESOLVES", Some("e1"))),
        "wave 129: an ASSET-RESOLVES finding on a placed object must route — the engine emits \
             these today, which is what made this the reachable half of the defect"
    );
    // And the affordance is the styling, not a name: the "yes" class carries pointer AND hover,
    // the "no" class carries neither.
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

/// With no router installed — the host build, and the editor before its mount — every row is
/// INERT. The safe direction: a panel that cannot ask does not claim.
#[test]
fn with_no_router_registered_every_row_renders_inert() {
    // A probe that resolves nothing stands in for "no probe": both are the `false` answer, and
    // this thread's thread_local is the panel's whole state.
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

/// One decision, one place. The row takes its classes from [`row_cursor_class`] and its
/// `clickable` from [`finding_is_routable`] — which asks the SHIPPED router — and no live code in
/// this panel decides clickability from the mere presence of an id.
#[test]
fn the_row_never_guesses_at_selectability() {
    let src = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/validation_panel.rs"
    )));
    let row = only_body(&src, &format!("fn finding{}", "_row_view"));
    assert!(
        row.contains(&format!("finding{}", "_is_routable(")),
        "wave 129: the row must decide clickability by asking the router"
    );
    assert!(
        row.contains(&format!("row{}", "_cursor_class(")),
        "wave 129: the row must take its cursor/hover classes from the one affordance function"
    );
    let routable = only_body(&src, &format!("fn finding{}", "_is_routable"));
    assert!(
        routable.contains(&format!("subject_id{}", "_routes")),
        "wave 129: clickability must be the ROUTER's resolution, not a second opinion about \
             which findings look selectable"
    );
    // NEGATIVE — the widest haystack in which the claim is even statable: the whole of this
    // file's LIVE code (the test module, which legitimately asserts the FACT `is_selectable`
    // states, is cut first). Any view or affordance code that calls it again goes red here.
    assert_eq!(
        src.matches(&format!(".is{}()", "_selectable")).count(),
        0,
        "wave 129: no live code in this panel may take `names an id` for `is clickable` — that \
             substitution IS the defect"
    );
    // Literals KEPT (the class text is code that ships), over the whole file's live half: the
    // affordance is spelled in exactly ONE place, so a second pointer class cannot appear beside
    // a hand-rolled guard.
    let lit = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/validation_panel.rs"
    )));
    assert_eq!(
        lit.matches(&format!("cursor{}", "-pointer")).count(),
        1,
        "wave 129: `cursor-pointer` belongs to `row_cursor_class` and nowhere else"
    );
    let row_lit = only_body(&lit, &format!("fn finding{}", "_row_view"));
    assert!(
        !row_lit.contains(&format!("cursor{}", "-pointer")),
        "wave 129: the row must not hand-roll the affordance beside the function that owns it"
    );
}
