use super::{inert_settings_row_reason, owner_is_routable, row_cursor_class, SettingOwner};
use crate::v2::apps::editor::ui::inspector::validation_panel::register_route_probe;
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

/// Mission-owned rows name no entity: they are never routable, wear no pointer, and carry an
/// explicit inert reason. Perturbation RED: make `inert_settings_row_reason(Mission)` return an
/// empty string, or make `owner_is_routable(Mission)` return true.
#[test]
fn a_mission_owned_row_is_inert_with_a_reason() {
    let owner = SettingOwner::Mission;
    register_route_probe(std::rc::Rc::new(|_: &str| true));
    assert!(
        !owner_is_routable(&owner),
        "T-758: Mission has no subject_id — even a probe that resolves everything must leave \
         it inert (there is nothing to select)"
    );
    assert!(
        !row_cursor_class(owner_is_routable(&owner)).contains("cursor-pointer"),
        "T-758: a mission-owned row must wear no pointer affordance"
    );
    let reason = inert_settings_row_reason(&owner);
    assert!(
        reason.to_lowercase().contains("no entity")
            || reason.to_lowercase().contains("not click"),
        "T-758: the inert reason must tell the author why the row is not a click target, got \
         {reason:?}"
    );
}

/// An entity row the probe refuses is the same inert shape as Mission — reason names the
/// refusal, not a hardcoded kind ban. Affordance stays glued to `subject_id_routes`.
#[test]
fn an_unroutable_entity_row_is_inert_with_a_reason() {
    register_route_probe(std::rc::Rc::new(|_: &str| false));
    let owner = SettingOwner::Entity {
        kind: "Zone",
        id: "z-circle".into(),
        label: "Play area".into(),
    };
    assert!(
        !owner_is_routable(&owner),
        "T-758: probe refusal ⇒ not clickable"
    );
    let reason = inert_settings_row_reason(&owner);
    assert!(
        reason.to_lowercase().contains("not selectable")
            || reason.to_lowercase().contains("resolves no"),
        "T-758: entity inert reason must name the router refusal, got {reason:?}"
    );
}

/// **THE shape pin.** `setting_row_view` must branch: a selectable row is a `<button>`; an
/// inert row is a non-focusable element carrying `aria-disabled` and
/// `inert_settings_row_reason`. Restoring the pre-T-758 "always `<button>`" shape makes this
/// red — that is the wave-115 MINOR.
///
/// Literals kept (`live_source`): the claim is about the attributes/tags that ship.
#[test]
fn an_inert_row_is_not_a_focusable_button() {
    let lit = live_source(include_str!("../settings_modal.rs"));
    let row = only_body(&lit, &format!("fn setting{}", "_row_view"));
    assert!(
        row.contains("if selectable") || row.contains("if clickable"),
        "T-758: the row must BRANCH on the same boolean that owns clickability — one shape for \
         live, one for inert"
    );
    assert!(
        row.contains("<button") && row.contains("</button>"),
        "T-758: a selectable row must still be a real button (keyboard activates the click)"
    );
    assert!(
        row.contains("<div")
            && row.contains("aria-disabled")
            && row.contains(&format!("inert{}", "_settings_row_reason")),
        "T-758: an inert row must be a non-focusable element carrying aria-disabled and the \
         reason — not a tab-stop button that does nothing"
    );
    // The inert branch must not be a disabled-looking button: no second `<button` after the
    // branch that would still focus. Count opening button tags inside the function — exactly
    // one (the selectable arm).
    let buttons = row.matches("<button").count();
    assert_eq!(
        buttons, 1,
        "T-758: exactly one <button> in setting_row_view (the selectable arm); an inert \
         <button aria-disabled> would still be a tab stop unless tabindex=-1 is also set, and \
         this slice chose the non-focusable element shape"
    );
}

/// Clickability is still the registered probe — never a kind list, never a fallback. This is
/// the T-754 invariant restated for the a11y slice so a "fix focus by hardcoding Mission as
/// inert and everything else as a button" cannot green the shape pin while reopening dead
/// clicks on unroutable entities.
#[test]
fn inert_shape_still_asks_subject_id_routes_not_a_kind_list() {
    let src = live_code(include_str!("../settings_modal.rs"));
    let routable = only_body(&src, &format!("fn owner{}", "_is_routable"));
    assert!(
        routable.contains(&format!("subject_id{}", "_routes")),
        "T-758: clickability must remain subject_id_routes — the shape follows that boolean, \
         it does not replace it"
    );
    let body = only_body(&src, &format!("fn render{}", "_all_settings_body"));
    assert!(
        body.contains(&format!("owner{}", "_is_routable(")),
        "T-758: the body must still decide clickable via owner_is_routable"
    );
    // NEGATIVE: no matches!(kind, …) / hardcoded selectable-kind table in the live affordance
    // path. `SettingOwner::Mission` in inert_settings_row_reason is a REASON branch, not a
    // clickability decision — pinned separately by asking owner_is_routable above.
    let row = only_body(&src, &format!("fn setting{}", "_row_view"));
    assert!(
        !row.contains("matches!") && !row.contains("DocKind::"),
        "T-758: setting_row_view must not hardcode kind lists for the element shape"
    );
}
