use super::{
    finding_is_routable, inert_finding_row_reason, register_route_probe, row_cursor_class,
    PanelFinding,
};
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};
use website_map_engine::data::scenario::validate::Primitive;
use website_map_engine::data::scenario::validate::Severity;

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

/// Positional / empty-id findings name nobody: inert with an explicit reason. Affordance stays
/// glued to [`finding_is_routable`].
#[test]
fn a_positional_finding_row_is_inert_with_a_reason() {
    register_route_probe(std::rc::Rc::new(|_: &str| true));
    let f = pf("V2-FACTION-MAX", None);
    assert!(
        !finding_is_routable(&f),
        "wave 132: a positional finding must stay inert even under an always-true probe"
    );
    assert!(
        !row_cursor_class(finding_is_routable(&f)).contains("cursor-pointer"),
        "wave 132: an inert finding must wear no pointer affordance"
    );
    let reason = inert_finding_row_reason(&f);
    assert!(
            reason.to_lowercase().contains("no selectable")
                || reason.to_lowercase().contains("nothing"),
            "wave 132: inert reason must tell the author why the row is not a click target, got              {reason:?}"
        );
}

/// Named subject the probe refuses — same inert shape; reason names the refusal.
#[test]
fn an_unroutable_finding_row_is_inert_with_a_reason() {
    register_route_probe(std::rc::Rc::new(|_: &str| false));
    let f = pf("ASSET-RESOLVES", Some("e1"));
    assert!(
        !finding_is_routable(&f),
        "wave 132: probe refusal ⇒ not clickable"
    );
    let reason = inert_finding_row_reason(&f);
    assert!(
        reason.to_lowercase().contains("not selectable")
            || reason.to_lowercase().contains("resolves no"),
        "wave 132: entity inert reason must name the router refusal, got {reason:?}"
    );
}

/// **THE shape pin.** `finding_row_view` must branch: selectable → `<button>`; inert →
/// non-focusable element with `aria-disabled` + `inert_finding_row_reason`. Restoring the
/// always-`<button>` shape makes this red (wave-115 MINOR class / T-758 peer).
#[test]
fn an_inert_finding_row_is_not_a_focusable_button() {
    let lit = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/validation_panel.rs"
    )));
    let row = only_body(&lit, &format!("fn finding{}", "_row_view"));
    assert!(
        row.contains("if selectable"),
        "wave 132: the row must BRANCH on the same boolean that owns clickability"
    );
    assert!(
        row.contains("<button") && row.contains("</button>"),
        "wave 132: a selectable finding must still be a real button"
    );
    assert!(
            row.contains("<div")
                && row.contains("aria-disabled")
                && row.contains(&format!("inert{}", "_finding_row_reason")),
            "wave 132: an inert finding must be a non-focusable element carrying aria-disabled and              the reason — not a tab-stop button that does nothing"
        );
    assert_eq!(
        row.matches("<button").count(),
        1,
        "wave 132: exactly one <button> in finding_row_view (the selectable arm)"
    );
}

/// Clickability remains the registered probe — shape follows that boolean, does not replace it.
#[test]
fn inert_finding_shape_still_asks_subject_id_routes() {
    let src = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/validation_panel.rs"
    )));
    let routable = only_body(&src, &format!("fn finding{}", "_is_routable"));
    assert!(
        routable.contains(&format!("subject_id{}", "_routes")),
        "wave 132: clickability must remain subject_id_routes"
    );
    let row = only_body(&src, &format!("fn finding{}", "_row_view"));
    assert!(
        row.contains(&format!("finding{}", "_is_routable(")),
        "wave 132: finding_row_view must still decide clickable via finding_is_routable"
    );
    assert!(
        !row.contains("matches!") && !row.contains("DocKind::"),
        "wave 132: finding_row_view must not hardcode kind lists for the element shape"
    );
}
